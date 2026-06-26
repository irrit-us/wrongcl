// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get navProxies => '代理';

  @override
  String get navProfiles => '配置档';

  @override
  String get navConnections => '连接';

  @override
  String get navRequests => '请求';

  @override
  String get navLogs => '日志';

  @override
  String get navSettings => '设置';

  @override
  String get navBasic => '基本';

  @override
  String get navNetwork => '网络';

  @override
  String get navDns => 'DNS';

  @override
  String get navAdvanced => '高级';

  @override
  String get navInspect => '检视';

  @override
  String get runtimeLabel => '运行时';

  @override
  String get runtimeWorking => '处理中...';

  @override
  String get runtimeRunning => '运行中';

  @override
  String get runtimeStopped => '已停止';

  @override
  String get runtimeStartTooltip => '启动';

  @override
  String get runtimeStopTooltip => '停止';

  @override
  String get modeGlobal => '全局';

  @override
  String get modeRule => '规则';

  @override
  String get modeDirect => '直连';

  @override
  String get modeAdd => '添加';

  @override
  String get modeAddTitle => '添加模式';

  @override
  String get modeNewUserMode => '新建用户模式';

  @override
  String get modeName => '名称';

  @override
  String get modeNameRequired => '请输入名称';

  @override
  String get modeNameConflictsBuiltin => '名称与内置模式冲突';

  @override
  String get modeProxy => '代理';

  @override
  String get modeScriptOptional => '脚本（可选）';

  @override
  String get modeNone => '— 无 —';

  @override
  String get modePickProxy => '请选择该模式要绑定的代理或分组。';

  @override
  String get commonClose => '关闭';

  @override
  String get commonCancel => '取消';

  @override
  String get commonSave => '保存';

  @override
  String get commonSavingEllipsis => '保存中…';

  @override
  String get commonLoadingEllipsis => '加载中...';

  @override
  String get commonApplyingEllipsis => '应用中…';

  @override
  String get trafficUp => '上行';

  @override
  String get trafficDown => '下行';

  @override
  String get trafficPeak => '峰值';

  @override
  String get trafficTotal => '总计';

  @override
  String get trafficAvg1Min => '近1分钟均值';

  @override
  String get windowKeepOnTop => '置顶';

  @override
  String get windowUnpinFromTop => '取消置顶';

  @override
  String get windowMinimize => '最小化';

  @override
  String get windowRestore => '还原';

  @override
  String get windowMaximize => '最大化';

  @override
  String get windowClose => '关闭';

  @override
  String get settingsAutostart => '开机自启';

  @override
  String get settingsAutostartLoading => '正在加载自启状态...';

  @override
  String get settingsEnableAutostart => '启用开机自启';

  @override
  String get settingsDisableAutostart => '禁用开机自启';

  @override
  String get settingsLanguage => '语言';

  @override
  String get settingsLanguageHint => '语言偏好保存在本地。';

  @override
  String get settingsAppLanguage => '应用语言';

  @override
  String get settingsTheme => '主题';

  @override
  String get settingsThemeHint => '主题模式和配色保存在本地。';

  @override
  String get settingsThemeMode => '主题模式';

  @override
  String get settingsThemeFollowSystem => '跟随系统';

  @override
  String get settingsThemeLight => '浅色';

  @override
  String get settingsThemeDark => '深色';

  @override
  String get settingsThemePalette => '主题配色';

  @override
  String get settingsLayout => '布局';

  @override
  String get settingsLayoutHint => '在从右向左阅读时（阿拉伯语、希伯来语等）将图标移到右侧。';

  @override
  String get settingsChipIconSide => '图标位置';

  @override
  String get settingsChipIconLeft => '左侧（默认）';

  @override
  String get settingsChipIconRight => '右侧（RTL）';

  @override
  String get networkLocalProxyListenAddress => '本地代理监听地址';

  @override
  String get networkListenHost => '监听主机';

  @override
  String get networkListenPort => '监听端口';

  @override
  String get networkSystemProxy => '系统代理';

  @override
  String get networkEnableSystemProxy => '启用系统代理';

  @override
  String get networkDisableSystemProxy => '禁用系统代理';

  @override
  String get networkTunSetup => 'TUN 配置';

  @override
  String get networkTunStatusAvailable => 'TUN 状态可用。';

  @override
  String get networkPrepareTunInterface => '准备 TUN 接口';

  @override
  String get networkRemovePreparedInterface => '移除已准备接口';

  @override
  String get networkMixedProtocolToggles => '混合协议开关';

  @override
  String get networkEnableSocks5Listener => '启用 SOCKS5 监听';

  @override
  String get networkEnableSocks5Subtitle => '在混合端口接受本地 SOCKS5 客户端。';

  @override
  String get networkEnableHttpProxyListener => '启用 HTTP 代理监听';

  @override
  String get networkEnableHttpProxySubtitle => '接受 HTTP CONNECT 和绝对形式的代理请求。';

  @override
  String get dnsResolverBackend => '解析后端';

  @override
  String get dnsApplyImmediately => '立即生效于当前运行时。';

  @override
  String get dnsApplyOnNextStart => '保存到当前草稿，下次启动时使用。';

  @override
  String get dnsBackend => '后端';

  @override
  String get dnsUdpServer => 'UDP 服务器';

  @override
  String get dnsDohUrl => 'DoH 网址';

  @override
  String get dnsHelperSystem => '使用宿主机系统解析器进行基于 IP 的路由决策。';

  @override
  String get dnsHelperUdp => '直接查询 DNS 服务器。udp://1.1.1.1:53 和 1.1.1.1:53 均可。';

  @override
  String get dnsHelperDoh => '使用 DNS over HTTPS 进行基于 IP 的路由决策。';

  @override
  String get dnsApplyDnsSettings => '应用 DNS 设置';

  @override
  String get advancedDiagnostics => '诊断';

  @override
  String get advancedRefreshStatus => '刷新状态';

  @override
  String get advancedValidateConfig => '校验配置';

  @override
  String get advancedLogLevel => '日志级别';

  @override
  String get advancedLogLevelHint => '此设置仅过滤日志页面的显示，目前不会影响原生日志输出级别。';

  @override
  String get advancedLogsPageFilter => '日志页过滤器';

  @override
  String advancedCurrentFilter(String label) {
    return '当前过滤：$label';
  }

  @override
  String get advancedRawConfigEditor => '原始配置编辑器';

  @override
  String get advancedRawConfigEditorHint =>
      '以 JSON 编辑当前草稿。文件导出支持 TOML，加载文件接受原生客户端可解析的任何格式。';

  @override
  String get advancedConfigFilePath => '配置文件路径';

  @override
  String get advancedLoadFile => '加载文件';

  @override
  String get advancedExportJson => '导出 JSON';

  @override
  String get advancedExportToml => '导出 TOML';

  @override
  String get advancedLoadCurrentDraft => '加载当前草稿';

  @override
  String get advancedRawClientConfigJson => '原始客户端配置（JSON）';

  @override
  String get advancedApplyJsonToDraft => '应用 JSON 到草稿';

  @override
  String get proxiesEmptyStopped => '启动代理后即可查看端点与分组。';

  @override
  String get proxiesEmptyNoEndpoints => '运行时尚未上报任何端点。代理完全启动后请刷新。';

  @override
  String get proxiesNoActiveSelection => '未选择激活项';

  @override
  String proxiesActiveLabel(String kind, String name) {
    return '激活$kind：$name';
  }

  @override
  String get proxiesEndpoints => '端点';

  @override
  String get proxiesAuto => '自动';

  @override
  String get proxiesUnknownEndpoint => '未知端点';

  @override
  String get connectionsCloseAll => '全部关闭';

  @override
  String get connectionsEmpty => '暂无活动连接。当流量经过本地代理时，活动连接会显示在此。';

  @override
  String connectionsVia(String app) {
    return '经由 $app';
  }

  @override
  String get requestsEmpty => '尚未捕获任何请求。在本地代理上发出流量后，最近的请求会显示在此。';

  @override
  String get logsNoMatch => '没有符合当前日志级别筛选的条目。';

  @override
  String get logsEmpty => '尚未捕获任何日志条目。代理运行时，运行时事件会实时显示在此。';

  @override
  String get profilesCurrentDraft => '当前草稿';

  @override
  String get profilesProfileName => '配置档名称';

  @override
  String get profilesWrongsvImport => 'wrongsv 导入';

  @override
  String get profilesWrongsvConfigPath => 'wrongsv 配置路径';

  @override
  String get profilesServerHost => '用于客户端的服务器地址';

  @override
  String get profilesLocalListenHost => '本地监听主机';

  @override
  String get profilesLocalListenPort => '本地监听端口';

  @override
  String get profilesInspectWrongsv => '检视 wrongsv';

  @override
  String get profilesAdaptWrongsv => '适配 wrongsv';

  @override
  String get profilesCompleteImport => '完成导入';

  @override
  String get profilesSavedProfiles => '已保存的配置档';

  @override
  String get profilesSavedEmpty => '尚未保存任何配置档。保存当前草稿即可建立可复用条目。';

  @override
  String get profilesLoadSelected => '加载所选';

  @override
  String get profilesDuplicateSelected => '复制所选';

  @override
  String get profilesDeleteSelected => '删除所选';

  @override
  String get profilesNew => '新建';

  @override
  String get profilesSaveCurrent => '保存当前';

  @override
  String get profilesDeleteTitle => '删除已保存的配置档？';

  @override
  String profilesDeleteMessage(String name) {
    return '从本地配置档列表中删除“$name”？这不会影响远端 wrongsv 服务器。';
  }

  @override
  String get profilesDeleteConfirm => '删除配置档';
}
