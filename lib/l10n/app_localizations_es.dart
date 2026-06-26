// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Spanish Castilian (`es`).
class AppLocalizationsEs extends AppLocalizations {
  AppLocalizationsEs([String locale = 'es']) : super(locale);

  @override
  String get navProxies => 'Proxies';

  @override
  String get navProfiles => 'Perfiles';

  @override
  String get navConnections => 'Conexiones';

  @override
  String get navRequests => 'Solicitudes';

  @override
  String get navLogs => 'Registros';

  @override
  String get navSettings => 'Ajustes';

  @override
  String get navBasic => 'Básico';

  @override
  String get navNetwork => 'Red';

  @override
  String get navDns => 'DNS';

  @override
  String get navAdvanced => 'Avanzado';

  @override
  String get navInspect => 'Inspeccionar';

  @override
  String get runtimeLabel => 'Tiempo de ejecución';

  @override
  String get runtimeWorking => 'Procesando...';

  @override
  String get runtimeRunning => 'En ejecución';

  @override
  String get runtimeStopped => 'Detenido';

  @override
  String get runtimeStartTooltip => 'Iniciar';

  @override
  String get runtimeStopTooltip => 'Detener';

  @override
  String get modeGlobal => 'Global';

  @override
  String get modeRule => 'Regla';

  @override
  String get modeDirect => 'Directo';

  @override
  String get modeAdd => 'Añadir';

  @override
  String get modeAddTitle => 'Añadir modo';

  @override
  String get modeNewUserMode => 'Nuevo modo de usuario';

  @override
  String get modeName => 'Nombre';

  @override
  String get modeNameRequired => 'El nombre es obligatorio';

  @override
  String get modeNameConflictsBuiltin =>
      'El nombre coincide con un modo integrado';

  @override
  String get modeProxy => 'Proxy';

  @override
  String get modeScriptOptional => 'Script (opcional)';

  @override
  String get modeNone => '— ninguno —';

  @override
  String get modePickProxy => 'Elige un proxy o grupo al que anclar este modo.';

  @override
  String get commonClose => 'Cerrar';

  @override
  String get commonCancel => 'Cancelar';

  @override
  String get commonSave => 'Guardar';

  @override
  String get commonSavingEllipsis => 'Guardando…';

  @override
  String get commonLoadingEllipsis => 'Cargando...';

  @override
  String get commonApplyingEllipsis => 'Aplicando…';

  @override
  String get trafficUp => 'Subida';

  @override
  String get trafficDown => 'Bajada';

  @override
  String get trafficPeak => 'Pico';

  @override
  String get trafficTotal => 'Total';

  @override
  String get trafficAvg1Min => 'Media (1 min)';

  @override
  String get windowKeepOnTop => 'Mantener encima';

  @override
  String get windowUnpinFromTop => 'Quitar de encima';

  @override
  String get windowMinimize => 'Minimizar';

  @override
  String get windowRestore => 'Restaurar';

  @override
  String get windowMaximize => 'Maximizar';

  @override
  String get windowClose => 'Cerrar';

  @override
  String get settingsAutostart => 'Inicio automático';

  @override
  String get settingsAutostartLoading =>
      'Cargando estado de inicio automático...';

  @override
  String get settingsEnableAutostart => 'Activar inicio automático';

  @override
  String get settingsDisableAutostart => 'Desactivar inicio automático';

  @override
  String get settingsLanguage => 'Idioma';

  @override
  String get settingsLanguageHint =>
      'La preferencia de idioma se guarda localmente.';

  @override
  String get settingsAppLanguage => 'Idioma de la app';

  @override
  String get settingsTheme => 'Tema';

  @override
  String get settingsThemeHint =>
      'El modo y la paleta del tema se guardan localmente.';

  @override
  String get settingsThemeMode => 'Modo de tema';

  @override
  String get settingsThemeFollowSystem => 'Seguir al sistema';

  @override
  String get settingsThemeLight => 'Claro';

  @override
  String get settingsThemeDark => 'Oscuro';

  @override
  String get settingsThemePalette => 'Paleta del tema';

  @override
  String get settingsLayout => 'Diseño';

  @override
  String get settingsLayoutHint =>
      'Mueve los iconos a la derecha al leer de derecha a izquierda (árabe, hebreo, ...).';

  @override
  String get settingsChipIconSide => 'Lado del icono';

  @override
  String get settingsChipIconLeft => 'Izquierda (predeterminado)';

  @override
  String get settingsChipIconRight => 'Derecha (RTL)';

  @override
  String get networkLocalProxyListenAddress =>
      'Dirección de escucha del proxy local';

  @override
  String get networkListenHost => 'Host de escucha';

  @override
  String get networkListenPort => 'Puerto de escucha';

  @override
  String get networkSystemProxy => 'Proxy del sistema';

  @override
  String get networkEnableSystemProxy => 'Activar proxy del sistema';

  @override
  String get networkDisableSystemProxy => 'Desactivar proxy del sistema';

  @override
  String get networkTunSetup => 'Configuración TUN';

  @override
  String get networkTunStatusAvailable => 'El estado TUN está disponible.';

  @override
  String get networkPrepareTunInterface => 'Preparar interfaz TUN';

  @override
  String get networkRemovePreparedInterface => 'Eliminar interfaz preparada';

  @override
  String get networkMixedProtocolToggles => 'Conmutadores de protocolos mixtos';

  @override
  String get networkEnableSocks5Listener => 'Activar escucha SOCKS5';

  @override
  String get networkEnableSocks5Subtitle =>
      'Acepta clientes SOCKS5 locales en el puerto mixto.';

  @override
  String get networkEnableHttpProxyListener => 'Activar escucha de proxy HTTP';

  @override
  String get networkEnableHttpProxySubtitle =>
      'Acepta peticiones HTTP CONNECT y de forma absoluta.';

  @override
  String get dnsResolverBackend => 'Backend de resolución';

  @override
  String get dnsApplyImmediately => 'Se aplica de inmediato al runtime activo.';

  @override
  String get dnsApplyOnNextStart =>
      'Se guarda en el borrador actual y se usa al próximo arranque.';

  @override
  String get dnsBackend => 'Backend';

  @override
  String get dnsUdpServer => 'Servidor UDP';

  @override
  String get dnsDohUrl => 'URL DoH';

  @override
  String get dnsHelperSystem =>
      'Usa el resolutor del SO anfitrión para decisiones de enrutamiento por IP.';

  @override
  String get dnsHelperUdp =>
      'Consulta un servidor DNS directamente. Se aceptan tanto udp://1.1.1.1:53 como 1.1.1.1:53.';

  @override
  String get dnsHelperDoh =>
      'Usa DNS sobre HTTPS para decisiones de enrutamiento por IP.';

  @override
  String get dnsApplyDnsSettings => 'Aplicar ajustes DNS';

  @override
  String get advancedDiagnostics => 'Diagnósticos';

  @override
  String get advancedRefreshStatus => 'Actualizar estado';

  @override
  String get advancedValidateConfig => 'Validar configuración';

  @override
  String get advancedLogLevel => 'Nivel de registro';

  @override
  String get advancedLogLevelHint =>
      'Filtra lo que muestra la página de registros. Aún no cambia el nivel de emisión del trazado nativo.';

  @override
  String get advancedLogsPageFilter => 'Filtro de la página de registros';

  @override
  String advancedCurrentFilter(String label) {
    return 'Filtro actual: $label';
  }

  @override
  String get advancedRawConfigEditor => 'Editor de configuración en bruto';

  @override
  String get advancedRawConfigEditorHint =>
      'Edita el borrador actual como JSON. TOML se admite para exportación; al cargar un archivo se acepta lo que el cliente nativo pueda interpretar.';

  @override
  String get advancedConfigFilePath => 'Ruta del archivo de configuración';

  @override
  String get advancedLoadFile => 'Cargar archivo';

  @override
  String get advancedExportJson => 'Exportar JSON';

  @override
  String get advancedExportToml => 'Exportar TOML';

  @override
  String get advancedLoadCurrentDraft => 'Cargar borrador actual';

  @override
  String get advancedRawClientConfigJson =>
      'Configuración del cliente en bruto (JSON)';

  @override
  String get advancedApplyJsonToDraft => 'Aplicar JSON al borrador';

  @override
  String get proxiesEmptyStopped =>
      'Inicia el proxy para inspeccionar endpoints y grupos.';

  @override
  String get proxiesEmptyNoEndpoints =>
      'Aún no se han reportado endpoints. Actualiza cuando el proxy esté completamente iniciado.';

  @override
  String get proxiesNoActiveSelection => 'Sin selección activa';

  @override
  String proxiesActiveLabel(String kind, String name) {
    return '$kind activo: $name';
  }

  @override
  String get proxiesEndpoints => 'Endpoints';

  @override
  String get proxiesAuto => 'auto';

  @override
  String get proxiesUnknownEndpoint => 'endpoint desconocido';

  @override
  String get connectionsCloseAll => 'Cerrar todas';

  @override
  String get connectionsEmpty =>
      'No hay conexiones activas. Las entradas en vivo aparecen aquí mientras el tráfico pasa por el proxy local.';

  @override
  String connectionsVia(String app) {
    return 'vía $app';
  }

  @override
  String get requestsEmpty =>
      'Aún no hay solicitudes capturadas. Envía tráfico por el proxy local y las solicitudes recientes aparecerán aquí.';

  @override
  String get logsNoMatch =>
      'Ninguna entrada coincide con el filtro de nivel actual.';

  @override
  String get logsEmpty =>
      'Aún no hay entradas de registro. Los eventos recientes se transmitirán aquí mientras el proxy esté activo.';

  @override
  String get profilesCurrentDraft => 'Borrador actual';

  @override
  String get profilesProfileName => 'Nombre del perfil';

  @override
  String get profilesWrongsvImport => 'Importar wrongsv';

  @override
  String get profilesWrongsvConfigPath => 'Ruta de configuración wrongsv';

  @override
  String get profilesServerHost =>
      'Host del servidor para la configuración adaptada';

  @override
  String get profilesLocalListenHost => 'Host de escucha local';

  @override
  String get profilesLocalListenPort => 'Puerto de escucha local';

  @override
  String get profilesInspectWrongsv => 'Inspeccionar wrongsv';

  @override
  String get profilesAdaptWrongsv => 'Adaptar wrongsv';

  @override
  String get profilesCompleteImport => 'Completar importación';

  @override
  String get profilesSavedProfiles => 'Perfiles guardados';

  @override
  String get profilesSavedEmpty =>
      'Aún no hay perfiles guardados. Guarda el borrador actual para crear una entrada reutilizable.';

  @override
  String get profilesLoadSelected => 'Cargar seleccionado';

  @override
  String get profilesDuplicateSelected => 'Duplicar seleccionado';

  @override
  String get profilesDeleteSelected => 'Eliminar seleccionado';

  @override
  String get profilesNew => 'Nuevo';

  @override
  String get profilesSaveCurrent => 'Guardar actual';

  @override
  String get profilesDeleteTitle => '¿Eliminar perfil guardado?';

  @override
  String profilesDeleteMessage(String name) {
    return '¿Eliminar «$name» de la lista local de perfiles? No se modifica el servidor wrongsv remoto.';
  }

  @override
  String get profilesDeleteConfirm => 'Eliminar perfil';
}
