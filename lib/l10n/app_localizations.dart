import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_ar.dart';
import 'app_localizations_en.dart';
import 'app_localizations_es.dart';
import 'app_localizations_fr.dart';
import 'app_localizations_zh.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('ar'),
    Locale('en'),
    Locale('es'),
    Locale('fr'),
    Locale('zh'),
  ];

  /// No description provided for @navProxies.
  ///
  /// In en, this message translates to:
  /// **'Proxies'**
  String get navProxies;

  /// No description provided for @navProfiles.
  ///
  /// In en, this message translates to:
  /// **'Profiles'**
  String get navProfiles;

  /// No description provided for @navConnections.
  ///
  /// In en, this message translates to:
  /// **'Connections'**
  String get navConnections;

  /// No description provided for @navRequests.
  ///
  /// In en, this message translates to:
  /// **'Requests'**
  String get navRequests;

  /// No description provided for @navLogs.
  ///
  /// In en, this message translates to:
  /// **'Logs'**
  String get navLogs;

  /// No description provided for @navSettings.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get navSettings;

  /// No description provided for @navBasic.
  ///
  /// In en, this message translates to:
  /// **'Basic'**
  String get navBasic;

  /// No description provided for @navNetwork.
  ///
  /// In en, this message translates to:
  /// **'Network'**
  String get navNetwork;

  /// No description provided for @navDns.
  ///
  /// In en, this message translates to:
  /// **'DNS'**
  String get navDns;

  /// No description provided for @navAdvanced.
  ///
  /// In en, this message translates to:
  /// **'Advanced'**
  String get navAdvanced;

  /// No description provided for @navInspect.
  ///
  /// In en, this message translates to:
  /// **'Inspect'**
  String get navInspect;

  /// No description provided for @runtimeLabel.
  ///
  /// In en, this message translates to:
  /// **'Runtime'**
  String get runtimeLabel;

  /// No description provided for @runtimeWorking.
  ///
  /// In en, this message translates to:
  /// **'Working...'**
  String get runtimeWorking;

  /// No description provided for @runtimeRunning.
  ///
  /// In en, this message translates to:
  /// **'Running'**
  String get runtimeRunning;

  /// No description provided for @runtimeStopped.
  ///
  /// In en, this message translates to:
  /// **'Stopped'**
  String get runtimeStopped;

  /// No description provided for @runtimeStartTooltip.
  ///
  /// In en, this message translates to:
  /// **'Start'**
  String get runtimeStartTooltip;

  /// No description provided for @runtimeStopTooltip.
  ///
  /// In en, this message translates to:
  /// **'Stop'**
  String get runtimeStopTooltip;

  /// No description provided for @modeGlobal.
  ///
  /// In en, this message translates to:
  /// **'Global'**
  String get modeGlobal;

  /// No description provided for @modeRule.
  ///
  /// In en, this message translates to:
  /// **'Rule'**
  String get modeRule;

  /// No description provided for @modeDirect.
  ///
  /// In en, this message translates to:
  /// **'Direct'**
  String get modeDirect;

  /// No description provided for @modeAdd.
  ///
  /// In en, this message translates to:
  /// **'Add'**
  String get modeAdd;

  /// No description provided for @modeAddTitle.
  ///
  /// In en, this message translates to:
  /// **'Add mode'**
  String get modeAddTitle;

  /// No description provided for @modeNewUserMode.
  ///
  /// In en, this message translates to:
  /// **'New user mode'**
  String get modeNewUserMode;

  /// No description provided for @modeName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get modeName;

  /// No description provided for @modeNameRequired.
  ///
  /// In en, this message translates to:
  /// **'Name is required'**
  String get modeNameRequired;

  /// No description provided for @modeNameConflictsBuiltin.
  ///
  /// In en, this message translates to:
  /// **'Name conflicts with a built-in mode'**
  String get modeNameConflictsBuiltin;

  /// No description provided for @modeProxy.
  ///
  /// In en, this message translates to:
  /// **'Proxy'**
  String get modeProxy;

  /// No description provided for @modeScriptOptional.
  ///
  /// In en, this message translates to:
  /// **'Script (optional)'**
  String get modeScriptOptional;

  /// No description provided for @modeNone.
  ///
  /// In en, this message translates to:
  /// **'— none —'**
  String get modeNone;

  /// No description provided for @modePickProxy.
  ///
  /// In en, this message translates to:
  /// **'Pick a proxy/group to pin this mode to.'**
  String get modePickProxy;

  /// No description provided for @commonClose.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get commonClose;

  /// No description provided for @commonCancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get commonCancel;

  /// No description provided for @commonSave.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get commonSave;

  /// No description provided for @commonSavingEllipsis.
  ///
  /// In en, this message translates to:
  /// **'Saving…'**
  String get commonSavingEllipsis;

  /// No description provided for @commonLoadingEllipsis.
  ///
  /// In en, this message translates to:
  /// **'Loading...'**
  String get commonLoadingEllipsis;

  /// No description provided for @commonApplyingEllipsis.
  ///
  /// In en, this message translates to:
  /// **'Applying…'**
  String get commonApplyingEllipsis;

  /// No description provided for @trafficUp.
  ///
  /// In en, this message translates to:
  /// **'Up'**
  String get trafficUp;

  /// No description provided for @trafficDown.
  ///
  /// In en, this message translates to:
  /// **'Down'**
  String get trafficDown;

  /// No description provided for @trafficPeak.
  ///
  /// In en, this message translates to:
  /// **'Peak'**
  String get trafficPeak;

  /// No description provided for @trafficTotal.
  ///
  /// In en, this message translates to:
  /// **'Total'**
  String get trafficTotal;

  /// No description provided for @trafficAvg1Min.
  ///
  /// In en, this message translates to:
  /// **'Avg(1min)'**
  String get trafficAvg1Min;

  /// No description provided for @windowKeepOnTop.
  ///
  /// In en, this message translates to:
  /// **'Keep on top'**
  String get windowKeepOnTop;

  /// No description provided for @windowUnpinFromTop.
  ///
  /// In en, this message translates to:
  /// **'Unpin from top'**
  String get windowUnpinFromTop;

  /// No description provided for @windowMinimize.
  ///
  /// In en, this message translates to:
  /// **'Minimize'**
  String get windowMinimize;

  /// No description provided for @windowRestore.
  ///
  /// In en, this message translates to:
  /// **'Restore'**
  String get windowRestore;

  /// No description provided for @windowMaximize.
  ///
  /// In en, this message translates to:
  /// **'Maximize'**
  String get windowMaximize;

  /// No description provided for @windowClose.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get windowClose;

  /// No description provided for @settingsAutostart.
  ///
  /// In en, this message translates to:
  /// **'Autostart'**
  String get settingsAutostart;

  /// No description provided for @settingsAutostartLoading.
  ///
  /// In en, this message translates to:
  /// **'Loading autostart status...'**
  String get settingsAutostartLoading;

  /// No description provided for @settingsEnableAutostart.
  ///
  /// In en, this message translates to:
  /// **'Enable autostart'**
  String get settingsEnableAutostart;

  /// No description provided for @settingsDisableAutostart.
  ///
  /// In en, this message translates to:
  /// **'Disable autostart'**
  String get settingsDisableAutostart;

  /// No description provided for @settingsLanguage.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get settingsLanguage;

  /// No description provided for @settingsLanguageHint.
  ///
  /// In en, this message translates to:
  /// **'Language preference is saved locally.'**
  String get settingsLanguageHint;

  /// No description provided for @settingsAppLanguage.
  ///
  /// In en, this message translates to:
  /// **'App language'**
  String get settingsAppLanguage;

  /// No description provided for @settingsTheme.
  ///
  /// In en, this message translates to:
  /// **'Theme'**
  String get settingsTheme;

  /// No description provided for @settingsThemeHint.
  ///
  /// In en, this message translates to:
  /// **'Theme mode and palette are saved locally.'**
  String get settingsThemeHint;

  /// No description provided for @settingsThemeMode.
  ///
  /// In en, this message translates to:
  /// **'Theme mode'**
  String get settingsThemeMode;

  /// No description provided for @settingsThemeFollowSystem.
  ///
  /// In en, this message translates to:
  /// **'Follow system'**
  String get settingsThemeFollowSystem;

  /// No description provided for @settingsThemeLight.
  ///
  /// In en, this message translates to:
  /// **'Light'**
  String get settingsThemeLight;

  /// No description provided for @settingsThemeDark.
  ///
  /// In en, this message translates to:
  /// **'Dark'**
  String get settingsThemeDark;

  /// No description provided for @settingsThemePalette.
  ///
  /// In en, this message translates to:
  /// **'Theme palette'**
  String get settingsThemePalette;

  /// No description provided for @settingsLayout.
  ///
  /// In en, this message translates to:
  /// **'Layout'**
  String get settingsLayout;

  /// No description provided for @settingsLayoutHint.
  ///
  /// In en, this message translates to:
  /// **'Move chip icons to the right when reading right-to-left (Arabic, Hebrew, ...).'**
  String get settingsLayoutHint;

  /// No description provided for @settingsChipIconSide.
  ///
  /// In en, this message translates to:
  /// **'Chip icon side'**
  String get settingsChipIconSide;

  /// No description provided for @settingsChipIconLeft.
  ///
  /// In en, this message translates to:
  /// **'Left (default)'**
  String get settingsChipIconLeft;

  /// No description provided for @settingsChipIconRight.
  ///
  /// In en, this message translates to:
  /// **'Right (RTL)'**
  String get settingsChipIconRight;

  /// No description provided for @networkLocalProxyListenAddress.
  ///
  /// In en, this message translates to:
  /// **'Local proxy listen address'**
  String get networkLocalProxyListenAddress;

  /// No description provided for @networkListenHost.
  ///
  /// In en, this message translates to:
  /// **'Listen host'**
  String get networkListenHost;

  /// No description provided for @networkListenPort.
  ///
  /// In en, this message translates to:
  /// **'Listen port'**
  String get networkListenPort;

  /// No description provided for @networkSystemProxy.
  ///
  /// In en, this message translates to:
  /// **'System proxy'**
  String get networkSystemProxy;

  /// No description provided for @networkEnableSystemProxy.
  ///
  /// In en, this message translates to:
  /// **'Enable system proxy'**
  String get networkEnableSystemProxy;

  /// No description provided for @networkDisableSystemProxy.
  ///
  /// In en, this message translates to:
  /// **'Disable system proxy'**
  String get networkDisableSystemProxy;

  /// No description provided for @networkTunSetup.
  ///
  /// In en, this message translates to:
  /// **'TUN setup'**
  String get networkTunSetup;

  /// No description provided for @networkTunStatusAvailable.
  ///
  /// In en, this message translates to:
  /// **'TUN status is available.'**
  String get networkTunStatusAvailable;

  /// No description provided for @networkPrepareTunInterface.
  ///
  /// In en, this message translates to:
  /// **'Prepare TUN interface'**
  String get networkPrepareTunInterface;

  /// No description provided for @networkRemovePreparedInterface.
  ///
  /// In en, this message translates to:
  /// **'Remove prepared interface'**
  String get networkRemovePreparedInterface;

  /// No description provided for @networkMixedProtocolToggles.
  ///
  /// In en, this message translates to:
  /// **'Mixed protocol toggles'**
  String get networkMixedProtocolToggles;

  /// No description provided for @networkEnableSocks5Listener.
  ///
  /// In en, this message translates to:
  /// **'Enable SOCKS5 listener'**
  String get networkEnableSocks5Listener;

  /// No description provided for @networkEnableSocks5Subtitle.
  ///
  /// In en, this message translates to:
  /// **'Accept local SOCKS5 clients on the mixed port.'**
  String get networkEnableSocks5Subtitle;

  /// No description provided for @networkEnableHttpProxyListener.
  ///
  /// In en, this message translates to:
  /// **'Enable HTTP proxy listener'**
  String get networkEnableHttpProxyListener;

  /// No description provided for @networkEnableHttpProxySubtitle.
  ///
  /// In en, this message translates to:
  /// **'Accept HTTP CONNECT and absolute-form proxy requests.'**
  String get networkEnableHttpProxySubtitle;

  /// No description provided for @dnsResolverBackend.
  ///
  /// In en, this message translates to:
  /// **'Resolver backend'**
  String get dnsResolverBackend;

  /// No description provided for @dnsApplyImmediately.
  ///
  /// In en, this message translates to:
  /// **'Applies to the active runtime immediately.'**
  String get dnsApplyImmediately;

  /// No description provided for @dnsApplyOnNextStart.
  ///
  /// In en, this message translates to:
  /// **'Saved into the current draft and used on the next start.'**
  String get dnsApplyOnNextStart;

  /// No description provided for @dnsBackend.
  ///
  /// In en, this message translates to:
  /// **'Backend'**
  String get dnsBackend;

  /// No description provided for @dnsUdpServer.
  ///
  /// In en, this message translates to:
  /// **'UDP server'**
  String get dnsUdpServer;

  /// No description provided for @dnsDohUrl.
  ///
  /// In en, this message translates to:
  /// **'DoH URL'**
  String get dnsDohUrl;

  /// No description provided for @dnsHelperSystem.
  ///
  /// In en, this message translates to:
  /// **'Use the host OS resolver for IP-based routing decisions.'**
  String get dnsHelperSystem;

  /// No description provided for @dnsHelperUdp.
  ///
  /// In en, this message translates to:
  /// **'Query a DNS server directly. Both udp://1.1.1.1:53 and 1.1.1.1:53 are accepted.'**
  String get dnsHelperUdp;

  /// No description provided for @dnsHelperDoh.
  ///
  /// In en, this message translates to:
  /// **'Use DNS over HTTPS for IP-based routing decisions.'**
  String get dnsHelperDoh;

  /// No description provided for @dnsApplyDnsSettings.
  ///
  /// In en, this message translates to:
  /// **'Apply DNS settings'**
  String get dnsApplyDnsSettings;

  /// No description provided for @advancedDiagnostics.
  ///
  /// In en, this message translates to:
  /// **'Diagnostics'**
  String get advancedDiagnostics;

  /// No description provided for @advancedRefreshStatus.
  ///
  /// In en, this message translates to:
  /// **'Refresh status'**
  String get advancedRefreshStatus;

  /// No description provided for @advancedValidateConfig.
  ///
  /// In en, this message translates to:
  /// **'Validate config'**
  String get advancedValidateConfig;

  /// No description provided for @advancedLogLevel.
  ///
  /// In en, this message translates to:
  /// **'Log level'**
  String get advancedLogLevel;

  /// No description provided for @advancedLogLevelHint.
  ///
  /// In en, this message translates to:
  /// **'This filters what the Logs page displays. It does not change the native tracing emission level yet.'**
  String get advancedLogLevelHint;

  /// No description provided for @advancedLogsPageFilter.
  ///
  /// In en, this message translates to:
  /// **'Logs page filter'**
  String get advancedLogsPageFilter;

  /// No description provided for @advancedCurrentFilter.
  ///
  /// In en, this message translates to:
  /// **'Current filter: {label}'**
  String advancedCurrentFilter(String label);

  /// No description provided for @advancedRawConfigEditor.
  ///
  /// In en, this message translates to:
  /// **'Raw config editor'**
  String get advancedRawConfigEditor;

  /// No description provided for @advancedRawConfigEditorHint.
  ///
  /// In en, this message translates to:
  /// **'Edit the current draft as JSON. TOML is supported for file export, while file loading accepts whatever the native client can parse.'**
  String get advancedRawConfigEditorHint;

  /// No description provided for @advancedConfigFilePath.
  ///
  /// In en, this message translates to:
  /// **'Config file path'**
  String get advancedConfigFilePath;

  /// No description provided for @advancedLoadFile.
  ///
  /// In en, this message translates to:
  /// **'Load file'**
  String get advancedLoadFile;

  /// No description provided for @advancedExportJson.
  ///
  /// In en, this message translates to:
  /// **'Export JSON'**
  String get advancedExportJson;

  /// No description provided for @advancedExportToml.
  ///
  /// In en, this message translates to:
  /// **'Export TOML'**
  String get advancedExportToml;

  /// No description provided for @advancedLoadCurrentDraft.
  ///
  /// In en, this message translates to:
  /// **'Load current draft'**
  String get advancedLoadCurrentDraft;

  /// No description provided for @advancedRawClientConfigJson.
  ///
  /// In en, this message translates to:
  /// **'Raw client config (JSON)'**
  String get advancedRawClientConfigJson;

  /// No description provided for @advancedApplyJsonToDraft.
  ///
  /// In en, this message translates to:
  /// **'Apply JSON to draft'**
  String get advancedApplyJsonToDraft;

  /// No description provided for @proxiesEmptyStopped.
  ///
  /// In en, this message translates to:
  /// **'Start the proxy to inspect endpoints and groups.'**
  String get proxiesEmptyStopped;

  /// No description provided for @proxiesEmptyNoEndpoints.
  ///
  /// In en, this message translates to:
  /// **'No endpoints reported by the runtime yet. Refresh once the proxy is fully started.'**
  String get proxiesEmptyNoEndpoints;

  /// No description provided for @proxiesNoActiveSelection.
  ///
  /// In en, this message translates to:
  /// **'No active selection'**
  String get proxiesNoActiveSelection;

  /// No description provided for @proxiesActiveLabel.
  ///
  /// In en, this message translates to:
  /// **'Active {kind}: {name}'**
  String proxiesActiveLabel(String kind, String name);

  /// No description provided for @proxiesEndpoints.
  ///
  /// In en, this message translates to:
  /// **'Endpoints'**
  String get proxiesEndpoints;

  /// No description provided for @proxiesAuto.
  ///
  /// In en, this message translates to:
  /// **'auto'**
  String get proxiesAuto;

  /// No description provided for @proxiesUnknownEndpoint.
  ///
  /// In en, this message translates to:
  /// **'unknown endpoint'**
  String get proxiesUnknownEndpoint;

  /// No description provided for @connectionsCloseAll.
  ///
  /// In en, this message translates to:
  /// **'Close all'**
  String get connectionsCloseAll;

  /// No description provided for @connectionsEmpty.
  ///
  /// In en, this message translates to:
  /// **'No active connections. Live entries appear here while traffic is flowing through the local proxy.'**
  String get connectionsEmpty;

  /// No description provided for @connectionsVia.
  ///
  /// In en, this message translates to:
  /// **'via {app}'**
  String connectionsVia(String app);

  /// No description provided for @requestsEmpty.
  ///
  /// In en, this message translates to:
  /// **'No captured requests yet. Send traffic through the local proxy and the most recent requests will appear here.'**
  String get requestsEmpty;

  /// No description provided for @logsNoMatch.
  ///
  /// In en, this message translates to:
  /// **'No log entries match the current log-level filter.'**
  String get logsNoMatch;

  /// No description provided for @logsEmpty.
  ///
  /// In en, this message translates to:
  /// **'No log entries captured yet. Recent runtime events will stream here while the proxy is active.'**
  String get logsEmpty;

  /// No description provided for @profilesCurrentDraft.
  ///
  /// In en, this message translates to:
  /// **'Current draft'**
  String get profilesCurrentDraft;

  /// No description provided for @profilesProfileName.
  ///
  /// In en, this message translates to:
  /// **'Profile name'**
  String get profilesProfileName;

  /// No description provided for @profilesWrongsvImport.
  ///
  /// In en, this message translates to:
  /// **'wrongsv import'**
  String get profilesWrongsvImport;

  /// No description provided for @profilesWrongsvConfigPath.
  ///
  /// In en, this message translates to:
  /// **'wrongsv config path'**
  String get profilesWrongsvConfigPath;

  /// No description provided for @profilesServerHost.
  ///
  /// In en, this message translates to:
  /// **'Server host for adapted client config'**
  String get profilesServerHost;

  /// No description provided for @profilesLocalListenHost.
  ///
  /// In en, this message translates to:
  /// **'Local listen host'**
  String get profilesLocalListenHost;

  /// No description provided for @profilesLocalListenPort.
  ///
  /// In en, this message translates to:
  /// **'Local listen port'**
  String get profilesLocalListenPort;

  /// No description provided for @profilesInspectWrongsv.
  ///
  /// In en, this message translates to:
  /// **'Inspect wrongsv'**
  String get profilesInspectWrongsv;

  /// No description provided for @profilesAdaptWrongsv.
  ///
  /// In en, this message translates to:
  /// **'Adapt wrongsv'**
  String get profilesAdaptWrongsv;

  /// No description provided for @profilesCompleteImport.
  ///
  /// In en, this message translates to:
  /// **'Complete import'**
  String get profilesCompleteImport;

  /// No description provided for @profilesSavedProfiles.
  ///
  /// In en, this message translates to:
  /// **'Saved profiles'**
  String get profilesSavedProfiles;

  /// No description provided for @profilesSavedEmpty.
  ///
  /// In en, this message translates to:
  /// **'No saved profiles yet. Save the current draft to create a reusable entry.'**
  String get profilesSavedEmpty;

  /// No description provided for @profilesLoadSelected.
  ///
  /// In en, this message translates to:
  /// **'Load selected'**
  String get profilesLoadSelected;

  /// No description provided for @profilesDuplicateSelected.
  ///
  /// In en, this message translates to:
  /// **'Duplicate selected'**
  String get profilesDuplicateSelected;

  /// No description provided for @profilesDeleteSelected.
  ///
  /// In en, this message translates to:
  /// **'Delete selected'**
  String get profilesDeleteSelected;

  /// No description provided for @profilesNew.
  ///
  /// In en, this message translates to:
  /// **'New'**
  String get profilesNew;

  /// No description provided for @profilesSaveCurrent.
  ///
  /// In en, this message translates to:
  /// **'Save current'**
  String get profilesSaveCurrent;

  /// No description provided for @profilesDeleteTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete saved profile?'**
  String get profilesDeleteTitle;

  /// No description provided for @profilesDeleteMessage.
  ///
  /// In en, this message translates to:
  /// **'Delete \"{name}\" from the local profile list? This does not change the remote wrongsv server.'**
  String profilesDeleteMessage(String name);

  /// No description provided for @profilesDeleteConfirm.
  ///
  /// In en, this message translates to:
  /// **'Delete profile'**
  String get profilesDeleteConfirm;
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['ar', 'en', 'es', 'fr', 'zh'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'ar':
      return AppLocalizationsAr();
    case 'en':
      return AppLocalizationsEn();
    case 'es':
      return AppLocalizationsEs();
    case 'fr':
      return AppLocalizationsFr();
    case 'zh':
      return AppLocalizationsZh();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
