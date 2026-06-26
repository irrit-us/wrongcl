// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Arabic (`ar`).
class AppLocalizationsAr extends AppLocalizations {
  AppLocalizationsAr([String locale = 'ar']) : super(locale);

  @override
  String get navProxies => 'الوكلاء';

  @override
  String get navProfiles => 'الملفات الشخصية';

  @override
  String get navConnections => 'الاتصالات';

  @override
  String get navRequests => 'الطلبات';

  @override
  String get navLogs => 'السجلات';

  @override
  String get navSettings => 'الإعدادات';

  @override
  String get navBasic => 'أساسي';

  @override
  String get navNetwork => 'الشبكة';

  @override
  String get navDns => 'DNS';

  @override
  String get navAdvanced => 'متقدم';

  @override
  String get navInspect => 'فحص';

  @override
  String get runtimeLabel => 'بيئة التشغيل';

  @override
  String get runtimeWorking => 'جارٍ المعالجة...';

  @override
  String get runtimeRunning => 'قيد التشغيل';

  @override
  String get runtimeStopped => 'متوقف';

  @override
  String get runtimeStartTooltip => 'تشغيل';

  @override
  String get runtimeStopTooltip => 'إيقاف';

  @override
  String get modeGlobal => 'شامل';

  @override
  String get modeRule => 'قاعدة';

  @override
  String get modeDirect => 'مباشر';

  @override
  String get modeAdd => 'إضافة';

  @override
  String get modeAddTitle => 'إضافة وضع';

  @override
  String get modeNewUserMode => 'وضع مستخدم جديد';

  @override
  String get modeName => 'الاسم';

  @override
  String get modeNameRequired => 'الاسم مطلوب';

  @override
  String get modeNameConflictsBuiltin => 'الاسم يتعارض مع وضع مدمج';

  @override
  String get modeProxy => 'الوكيل';

  @override
  String get modeScriptOptional => 'السكربت (اختياري)';

  @override
  String get modeNone => '— لا شيء —';

  @override
  String get modePickProxy => 'اختر وكيلاً أو مجموعة لربط هذا الوضع بها.';

  @override
  String get commonClose => 'إغلاق';

  @override
  String get commonCancel => 'إلغاء';

  @override
  String get commonSave => 'حفظ';

  @override
  String get commonSavingEllipsis => 'جارٍ الحفظ…';

  @override
  String get commonLoadingEllipsis => 'جارٍ التحميل...';

  @override
  String get commonApplyingEllipsis => 'جارٍ التطبيق…';

  @override
  String get trafficUp => 'الإرسال';

  @override
  String get trafficDown => 'الاستقبال';

  @override
  String get trafficPeak => 'الذروة';

  @override
  String get trafficTotal => 'الإجمالي';

  @override
  String get trafficAvg1Min => 'المعدل (دقيقة)';

  @override
  String get windowKeepOnTop => 'إبقاء فوق النوافذ';

  @override
  String get windowUnpinFromTop => 'إلغاء التثبيت في الأعلى';

  @override
  String get windowMinimize => 'تصغير';

  @override
  String get windowRestore => 'استعادة';

  @override
  String get windowMaximize => 'تكبير';

  @override
  String get windowClose => 'إغلاق';

  @override
  String get settingsAutostart => 'التشغيل التلقائي';

  @override
  String get settingsAutostartLoading => 'جارٍ تحميل حالة التشغيل التلقائي...';

  @override
  String get settingsEnableAutostart => 'تفعيل التشغيل التلقائي';

  @override
  String get settingsDisableAutostart => 'إيقاف التشغيل التلقائي';

  @override
  String get settingsLanguage => 'اللغة';

  @override
  String get settingsLanguageHint => 'يُحفظ تفضيل اللغة محلياً.';

  @override
  String get settingsAppLanguage => 'لغة التطبيق';

  @override
  String get settingsTheme => 'السمة';

  @override
  String get settingsThemeHint => 'يُحفظ وضع السمة ولوحة الألوان محلياً.';

  @override
  String get settingsThemeMode => 'وضع السمة';

  @override
  String get settingsThemeFollowSystem => 'حسب النظام';

  @override
  String get settingsThemeLight => 'فاتح';

  @override
  String get settingsThemeDark => 'داكن';

  @override
  String get settingsThemePalette => 'لوحة الألوان';

  @override
  String get settingsLayout => 'التخطيط';

  @override
  String get settingsLayoutHint =>
      'ضع أيقونات الشرائح على اليمين عند القراءة من اليمين إلى اليسار (العربية والعبرية ...).';

  @override
  String get settingsChipIconSide => 'موضع الأيقونة';

  @override
  String get settingsChipIconLeft => 'يسار (افتراضي)';

  @override
  String get settingsChipIconRight => 'يمين (RTL)';

  @override
  String get networkLocalProxyListenAddress => 'عنوان استماع الوكيل المحلي';

  @override
  String get networkListenHost => 'مضيف الاستماع';

  @override
  String get networkListenPort => 'منفذ الاستماع';

  @override
  String get networkSystemProxy => 'وكيل النظام';

  @override
  String get networkEnableSystemProxy => 'تفعيل وكيل النظام';

  @override
  String get networkDisableSystemProxy => 'إيقاف وكيل النظام';

  @override
  String get networkTunSetup => 'إعداد TUN';

  @override
  String get networkTunStatusAvailable => 'حالة TUN متاحة.';

  @override
  String get networkPrepareTunInterface => 'تجهيز واجهة TUN';

  @override
  String get networkRemovePreparedInterface => 'إزالة الواجهة المجهزة';

  @override
  String get networkMixedProtocolToggles => 'مفاتيح البروتوكول المختلط';

  @override
  String get networkEnableSocks5Listener => 'تفعيل مستمع SOCKS5';

  @override
  String get networkEnableSocks5Subtitle =>
      'قبول عملاء SOCKS5 المحليين على المنفذ المختلط.';

  @override
  String get networkEnableHttpProxyListener => 'تفعيل مستمع وكيل HTTP';

  @override
  String get networkEnableHttpProxySubtitle =>
      'قبول طلبات HTTP CONNECT والصيغة المطلقة.';

  @override
  String get dnsResolverBackend => 'خلفية المُحلِّل';

  @override
  String get dnsApplyImmediately => 'يُطبَّق فوراً على بيئة التشغيل النشطة.';

  @override
  String get dnsApplyOnNextStart =>
      'يُحفظ في المسودة الحالية ويُستخدم في التشغيل التالي.';

  @override
  String get dnsBackend => 'الخلفية';

  @override
  String get dnsUdpServer => 'خادم UDP';

  @override
  String get dnsDohUrl => 'عنوان DoH';

  @override
  String get dnsHelperSystem =>
      'استخدم مُحلِّل نظام التشغيل لقرارات التوجيه القائمة على IP.';

  @override
  String get dnsHelperUdp =>
      'استعلام خادم DNS مباشرة. كلٌّ من udp://1.1.1.1:53 و1.1.1.1:53 مقبول.';

  @override
  String get dnsHelperDoh =>
      'استخدم DNS عبر HTTPS لقرارات التوجيه القائمة على IP.';

  @override
  String get dnsApplyDnsSettings => 'تطبيق إعدادات DNS';

  @override
  String get advancedDiagnostics => 'التشخيص';

  @override
  String get advancedRefreshStatus => 'تحديث الحالة';

  @override
  String get advancedValidateConfig => 'التحقق من الإعدادات';

  @override
  String get advancedLogLevel => 'مستوى السجل';

  @override
  String get advancedLogLevelHint =>
      'يُصفّي ما يظهر في صفحة السجلات فقط، ولا يُغيّر مستوى الإخراج الأصلي حالياً.';

  @override
  String get advancedLogsPageFilter => 'مرشح صفحة السجلات';

  @override
  String advancedCurrentFilter(String label) {
    return 'المرشح الحالي: $label';
  }

  @override
  String get advancedRawConfigEditor => 'محرر الإعدادات الخام';

  @override
  String get advancedRawConfigEditorHint =>
      'حرّر المسودة الحالية بتنسيق JSON. يدعم التصدير TOML، ويقبل التحميل أي صيغة يقرأها العميل الأصلي.';

  @override
  String get advancedConfigFilePath => 'مسار ملف الإعدادات';

  @override
  String get advancedLoadFile => 'تحميل الملف';

  @override
  String get advancedExportJson => 'تصدير JSON';

  @override
  String get advancedExportToml => 'تصدير TOML';

  @override
  String get advancedLoadCurrentDraft => 'تحميل المسودة الحالية';

  @override
  String get advancedRawClientConfigJson => 'إعدادات العميل الخام (JSON)';

  @override
  String get advancedApplyJsonToDraft => 'تطبيق JSON على المسودة';

  @override
  String get proxiesEmptyStopped => 'ابدأ الوكيل لفحص نقاط النهاية والمجموعات.';

  @override
  String get proxiesEmptyNoEndpoints =>
      'لم تُبلَّغ نقاط نهاية بعد. يرجى التحديث عند اكتمال تشغيل الوكيل.';

  @override
  String get proxiesNoActiveSelection => 'لا يوجد تحديد نشط';

  @override
  String proxiesActiveLabel(String kind, String name) {
    return '$kind النشط: $name';
  }

  @override
  String get proxiesEndpoints => 'نقاط النهاية';

  @override
  String get proxiesAuto => 'تلقائي';

  @override
  String get proxiesUnknownEndpoint => 'نقطة نهاية غير معروفة';

  @override
  String get connectionsCloseAll => 'إغلاق الكل';

  @override
  String get connectionsEmpty =>
      'لا توجد اتصالات نشطة. تظهر الإدخالات الحية هنا أثناء تدفق الحركة عبر الوكيل المحلي.';

  @override
  String connectionsVia(String app) {
    return 'عبر $app';
  }

  @override
  String get requestsEmpty =>
      'لم تُلتقط طلبات بعد. أرسل حركة المرور عبر الوكيل المحلي وستظهر آخر الطلبات هنا.';

  @override
  String get logsNoMatch => 'لا توجد إدخالات مطابقة لمرشح المستوى الحالي.';

  @override
  String get logsEmpty =>
      'لم تُلتقط أي إدخالات بعد. ستُبث الأحداث الأخيرة هنا أثناء تشغيل الوكيل.';

  @override
  String get profilesCurrentDraft => 'المسودة الحالية';

  @override
  String get profilesProfileName => 'اسم الملف الشخصي';

  @override
  String get profilesWrongsvImport => 'استيراد wrongsv';

  @override
  String get profilesWrongsvConfigPath => 'مسار إعدادات wrongsv';

  @override
  String get profilesServerHost => 'مضيف الخادم للإعدادات المُكيَّفة';

  @override
  String get profilesLocalListenHost => 'مضيف الاستماع المحلي';

  @override
  String get profilesLocalListenPort => 'منفذ الاستماع المحلي';

  @override
  String get profilesInspectWrongsv => 'فحص wrongsv';

  @override
  String get profilesAdaptWrongsv => 'تكييف wrongsv';

  @override
  String get profilesCompleteImport => 'إكمال الاستيراد';

  @override
  String get profilesSavedProfiles => 'الملفات المحفوظة';

  @override
  String get profilesSavedEmpty =>
      'لا توجد ملفات محفوظة بعد. احفظ المسودة الحالية لإنشاء إدخال قابل لإعادة الاستخدام.';

  @override
  String get profilesLoadSelected => 'تحميل المحدد';

  @override
  String get profilesDuplicateSelected => 'تكرار المحدد';

  @override
  String get profilesDeleteSelected => 'حذف المحدد';

  @override
  String get profilesNew => 'جديد';

  @override
  String get profilesSaveCurrent => 'حفظ الحالي';

  @override
  String get profilesDeleteTitle => 'حذف الملف الشخصي المحفوظ؟';

  @override
  String profilesDeleteMessage(String name) {
    return 'حذف «$name» من قائمة الملفات المحلية؟ لا يؤثر ذلك على خادم wrongsv البعيد.';
  }

  @override
  String get profilesDeleteConfirm => 'حذف الملف';
}
