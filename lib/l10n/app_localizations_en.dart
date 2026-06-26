// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get navProxies => 'Proxies';

  @override
  String get navProfiles => 'Profiles';

  @override
  String get navConnections => 'Connections';

  @override
  String get navRequests => 'Requests';

  @override
  String get navLogs => 'Logs';

  @override
  String get navSettings => 'Settings';

  @override
  String get navBasic => 'Basic';

  @override
  String get navNetwork => 'Network';

  @override
  String get navDns => 'DNS';

  @override
  String get navAdvanced => 'Advanced';

  @override
  String get navInspect => 'Inspect';

  @override
  String get runtimeLabel => 'Runtime';

  @override
  String get runtimeWorking => 'Working...';

  @override
  String get runtimeRunning => 'Running';

  @override
  String get runtimeStopped => 'Stopped';

  @override
  String get runtimeStartTooltip => 'Start';

  @override
  String get runtimeStopTooltip => 'Stop';

  @override
  String get modeGlobal => 'Global';

  @override
  String get modeRule => 'Rule';

  @override
  String get modeDirect => 'Direct';

  @override
  String get modeAdd => 'Add';

  @override
  String get modeAddTitle => 'Add mode';

  @override
  String get modeNewUserMode => 'New user mode';

  @override
  String get modeName => 'Name';

  @override
  String get modeNameRequired => 'Name is required';

  @override
  String get modeNameConflictsBuiltin => 'Name conflicts with a built-in mode';

  @override
  String get modeProxy => 'Proxy';

  @override
  String get modeScriptOptional => 'Script (optional)';

  @override
  String get modeNone => '— none —';

  @override
  String get modePickProxy => 'Pick a proxy/group to pin this mode to.';

  @override
  String get commonClose => 'Close';

  @override
  String get commonCancel => 'Cancel';

  @override
  String get commonSave => 'Save';

  @override
  String get commonSavingEllipsis => 'Saving…';

  @override
  String get commonLoadingEllipsis => 'Loading...';

  @override
  String get commonApplyingEllipsis => 'Applying…';

  @override
  String get trafficUp => 'Up';

  @override
  String get trafficDown => 'Down';

  @override
  String get trafficPeak => 'Peak';

  @override
  String get trafficTotal => 'Total';

  @override
  String get trafficAvg1Min => 'Avg(1min)';

  @override
  String get windowKeepOnTop => 'Keep on top';

  @override
  String get windowUnpinFromTop => 'Unpin from top';

  @override
  String get windowMinimize => 'Minimize';

  @override
  String get windowRestore => 'Restore';

  @override
  String get windowMaximize => 'Maximize';

  @override
  String get windowClose => 'Close';

  @override
  String get settingsAutostart => 'Autostart';

  @override
  String get settingsAutostartLoading => 'Loading autostart status...';

  @override
  String get settingsEnableAutostart => 'Enable autostart';

  @override
  String get settingsDisableAutostart => 'Disable autostart';

  @override
  String get settingsLanguage => 'Language';

  @override
  String get settingsLanguageHint => 'Language preference is saved locally.';

  @override
  String get settingsAppLanguage => 'App language';

  @override
  String get settingsTheme => 'Theme';

  @override
  String get settingsThemeHint => 'Theme mode and palette are saved locally.';

  @override
  String get settingsThemeMode => 'Theme mode';

  @override
  String get settingsThemeFollowSystem => 'Follow system';

  @override
  String get settingsThemeLight => 'Light';

  @override
  String get settingsThemeDark => 'Dark';

  @override
  String get settingsThemePalette => 'Theme palette';

  @override
  String get settingsLayout => 'Layout';

  @override
  String get settingsLayoutHint =>
      'Move chip icons to the right when reading right-to-left (Arabic, Hebrew, ...).';

  @override
  String get settingsChipIconSide => 'Chip icon side';

  @override
  String get settingsChipIconLeft => 'Left (default)';

  @override
  String get settingsChipIconRight => 'Right (RTL)';

  @override
  String get networkLocalProxyListenAddress => 'Local proxy listen address';

  @override
  String get networkListenHost => 'Listen host';

  @override
  String get networkListenPort => 'Listen port';

  @override
  String get networkSystemProxy => 'System proxy';

  @override
  String get networkEnableSystemProxy => 'Enable system proxy';

  @override
  String get networkDisableSystemProxy => 'Disable system proxy';

  @override
  String get networkTunSetup => 'TUN setup';

  @override
  String get networkTunStatusAvailable => 'TUN status is available.';

  @override
  String get networkPrepareTunInterface => 'Prepare TUN interface';

  @override
  String get networkRemovePreparedInterface => 'Remove prepared interface';

  @override
  String get networkMixedProtocolToggles => 'Mixed protocol toggles';

  @override
  String get networkEnableSocks5Listener => 'Enable SOCKS5 listener';

  @override
  String get networkEnableSocks5Subtitle =>
      'Accept local SOCKS5 clients on the mixed port.';

  @override
  String get networkEnableHttpProxyListener => 'Enable HTTP proxy listener';

  @override
  String get networkEnableHttpProxySubtitle =>
      'Accept HTTP CONNECT and absolute-form proxy requests.';

  @override
  String get dnsResolverBackend => 'Resolver backend';

  @override
  String get dnsApplyImmediately =>
      'Applies to the active runtime immediately.';

  @override
  String get dnsApplyOnNextStart =>
      'Saved into the current draft and used on the next start.';

  @override
  String get dnsBackend => 'Backend';

  @override
  String get dnsUdpServer => 'UDP server';

  @override
  String get dnsDohUrl => 'DoH URL';

  @override
  String get dnsHelperSystem =>
      'Use the host OS resolver for IP-based routing decisions.';

  @override
  String get dnsHelperUdp =>
      'Query a DNS server directly. Both udp://1.1.1.1:53 and 1.1.1.1:53 are accepted.';

  @override
  String get dnsHelperDoh =>
      'Use DNS over HTTPS for IP-based routing decisions.';

  @override
  String get dnsApplyDnsSettings => 'Apply DNS settings';

  @override
  String get advancedDiagnostics => 'Diagnostics';

  @override
  String get advancedRefreshStatus => 'Refresh status';

  @override
  String get advancedValidateConfig => 'Validate config';

  @override
  String get advancedLogLevel => 'Log level';

  @override
  String get advancedLogLevelHint =>
      'This filters what the Logs page displays. It does not change the native tracing emission level yet.';

  @override
  String get advancedLogsPageFilter => 'Logs page filter';

  @override
  String advancedCurrentFilter(String label) {
    return 'Current filter: $label';
  }

  @override
  String get advancedRawConfigEditor => 'Raw config editor';

  @override
  String get advancedRawConfigEditorHint =>
      'Edit the current draft as JSON. TOML is supported for file export, while file loading accepts whatever the native client can parse.';

  @override
  String get advancedConfigFilePath => 'Config file path';

  @override
  String get advancedLoadFile => 'Load file';

  @override
  String get advancedExportJson => 'Export JSON';

  @override
  String get advancedExportToml => 'Export TOML';

  @override
  String get advancedLoadCurrentDraft => 'Load current draft';

  @override
  String get advancedRawClientConfigJson => 'Raw client config (JSON)';

  @override
  String get advancedApplyJsonToDraft => 'Apply JSON to draft';

  @override
  String get proxiesEmptyStopped =>
      'Start the proxy to inspect endpoints and groups.';

  @override
  String get proxiesEmptyNoEndpoints =>
      'No endpoints reported by the runtime yet. Refresh once the proxy is fully started.';

  @override
  String get proxiesNoActiveSelection => 'No active selection';

  @override
  String proxiesActiveLabel(String kind, String name) {
    return 'Active $kind: $name';
  }

  @override
  String get proxiesEndpoints => 'Endpoints';

  @override
  String get proxiesAuto => 'auto';

  @override
  String get proxiesUnknownEndpoint => 'unknown endpoint';

  @override
  String get connectionsCloseAll => 'Close all';

  @override
  String get connectionsEmpty =>
      'No active connections. Live entries appear here while traffic is flowing through the local proxy.';

  @override
  String connectionsVia(String app) {
    return 'via $app';
  }

  @override
  String get requestsEmpty =>
      'No captured requests yet. Send traffic through the local proxy and the most recent requests will appear here.';

  @override
  String get logsNoMatch =>
      'No log entries match the current log-level filter.';

  @override
  String get logsEmpty =>
      'No log entries captured yet. Recent runtime events will stream here while the proxy is active.';

  @override
  String get profilesCurrentDraft => 'Current draft';

  @override
  String get profilesProfileName => 'Profile name';

  @override
  String get profilesWrongsvImport => 'wrongsv import';

  @override
  String get profilesWrongsvConfigPath => 'wrongsv config path';

  @override
  String get profilesServerHost => 'Server host for adapted client config';

  @override
  String get profilesLocalListenHost => 'Local listen host';

  @override
  String get profilesLocalListenPort => 'Local listen port';

  @override
  String get profilesInspectWrongsv => 'Inspect wrongsv';

  @override
  String get profilesAdaptWrongsv => 'Adapt wrongsv';

  @override
  String get profilesCompleteImport => 'Complete import';

  @override
  String get profilesSavedProfiles => 'Saved profiles';

  @override
  String get profilesSavedEmpty =>
      'No saved profiles yet. Save the current draft to create a reusable entry.';

  @override
  String get profilesLoadSelected => 'Load selected';

  @override
  String get profilesDuplicateSelected => 'Duplicate selected';

  @override
  String get profilesDeleteSelected => 'Delete selected';

  @override
  String get profilesNew => 'New';

  @override
  String get profilesSaveCurrent => 'Save current';

  @override
  String get profilesDeleteTitle => 'Delete saved profile?';

  @override
  String profilesDeleteMessage(String name) {
    return 'Delete \"$name\" from the local profile list? This does not change the remote wrongsv server.';
  }

  @override
  String get profilesDeleteConfirm => 'Delete profile';
}
