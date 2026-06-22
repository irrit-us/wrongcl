$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
$ReleaseDir = Join-Path $RootDir "build\windows\x64\runner\Release"
$NativeDll = Join-Path $ReleaseDir "wrongcl_native.dll"
$WintunDll = if ($env:WRONGCL_WINTUN_DLL) {
  $env:WRONGCL_WINTUN_DLL
} else {
  Join-Path $ReleaseDir "wintun.dll"
}
$OutputPath = if ($args.Count -gt 0) { $args[0] } else { Join-Path $RootDir ".tmp\windows-tun-smoke.json" }
$InterfaceName = "wrongcl-smoke0"
$RoutePrefix = "198.18.0.0/15"
$Config = @{
  interface_name = $InterfaceName
  address_cidr = "198.18.0.1/15"
  routes = @($RoutePrefix)
  proxy_host = "127.0.0.1"
  proxy_port = 1080
} | ConvertTo-Json -Compress

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutputPath) | Out-Null

if (-not (Test-Path $NativeDll)) {
  throw "wrongcl_native.dll was not found at $NativeDll"
}
if (-not (Test-Path $WintunDll)) {
  throw "wintun.dll was not found at $WintunDll"
}

$env:WRONGCL_WINTUN_DLL = (Resolve-Path $WintunDll).Path

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class WrongclNativeSmoke {
  [DllImport(@"$NativeDll", CallingConvention = CallingConvention.Cdecl)]
  public static extern IntPtr wrongcl_tun_status_json();
  [DllImport(@"$NativeDll", CallingConvention = CallingConvention.Cdecl)]
  public static extern IntPtr wrongcl_tun_enable_json(IntPtr configJson);
  [DllImport(@"$NativeDll", CallingConvention = CallingConvention.Cdecl)]
  public static extern IntPtr wrongcl_tun_disable();
  [DllImport(@"$NativeDll", CallingConvention = CallingConvention.Cdecl)]
  public static extern void wrongcl_free_string(IntPtr ptr);
}
"@

function Read-NativeJson([IntPtr]$ptr) {
  if ($ptr -eq [IntPtr]::Zero) {
    throw "native returned null pointer"
  }
  try {
    return [Runtime.InteropServices.Marshal]::PtrToStringAnsi($ptr) | ConvertFrom-Json
  } finally {
    [WrongclNativeSmoke]::wrongcl_free_string($ptr)
  }
}

function Invoke-TunEnable([string]$json) {
  $configPtr = [Runtime.InteropServices.Marshal]::StringToHGlobalAnsi($json)
  try {
    return Read-NativeJson ([WrongclNativeSmoke]::wrongcl_tun_enable_json($configPtr))
  } finally {
    [Runtime.InteropServices.Marshal]::FreeHGlobal($configPtr)
  }
}

function Get-RouteSnapshot {
  @(Get-NetRoute -DestinationPrefix $RoutePrefix -ErrorAction SilentlyContinue | Select-Object DestinationPrefix, InterfaceIndex, NextHop, RouteMetric, State)
}

$result = [ordered]@{
  ran_at = (Get-Date).ToString("o")
  elevated = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
  wintun_dll = $env:WRONGCL_WINTUN_DLL
  before = $null
  enable = $null
  after_enable = $null
  routes_after_enable = @()
  disable = $null
  after_disable = $null
  routes_after_disable = @()
}

try {
  $result.before = Read-NativeJson ([WrongclNativeSmoke]::wrongcl_tun_status_json())
  $result.enable = Invoke-TunEnable $Config
  $result.after_enable = Read-NativeJson ([WrongclNativeSmoke]::wrongcl_tun_status_json())
  $result.routes_after_enable = Get-RouteSnapshot
} finally {
  $result.disable = Read-NativeJson ([WrongclNativeSmoke]::wrongcl_tun_disable())
  Start-Sleep -Milliseconds 300
  $result.after_disable = Read-NativeJson ([WrongclNativeSmoke]::wrongcl_tun_status_json())
  $result.routes_after_disable = Get-RouteSnapshot
}

$result | ConvertTo-Json -Depth 6 | Set-Content -Path $OutputPath -Encoding UTF8
Write-Output $OutputPath
