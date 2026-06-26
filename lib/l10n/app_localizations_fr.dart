// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for French (`fr`).
class AppLocalizationsFr extends AppLocalizations {
  AppLocalizationsFr([String locale = 'fr']) : super(locale);

  @override
  String get navProxies => 'Proxys';

  @override
  String get navProfiles => 'Profils';

  @override
  String get navConnections => 'Connexions';

  @override
  String get navRequests => 'Requêtes';

  @override
  String get navLogs => 'Journaux';

  @override
  String get navSettings => 'Paramètres';

  @override
  String get navBasic => 'Basique';

  @override
  String get navNetwork => 'Réseau';

  @override
  String get navDns => 'DNS';

  @override
  String get navAdvanced => 'Avancé';

  @override
  String get navInspect => 'Inspecter';

  @override
  String get runtimeLabel => 'Exécution';

  @override
  String get runtimeWorking => 'Traitement...';

  @override
  String get runtimeRunning => 'En marche';

  @override
  String get runtimeStopped => 'Arrêté';

  @override
  String get runtimeStartTooltip => 'Démarrer';

  @override
  String get runtimeStopTooltip => 'Arrêter';

  @override
  String get modeGlobal => 'Global';

  @override
  String get modeRule => 'Règle';

  @override
  String get modeDirect => 'Direct';

  @override
  String get modeAdd => 'Ajouter';

  @override
  String get modeAddTitle => 'Ajouter un mode';

  @override
  String get modeNewUserMode => 'Nouveau mode utilisateur';

  @override
  String get modeName => 'Nom';

  @override
  String get modeNameRequired => 'Le nom est obligatoire';

  @override
  String get modeNameConflictsBuiltin =>
      'Le nom entre en conflit avec un mode intégré';

  @override
  String get modeProxy => 'Proxy';

  @override
  String get modeScriptOptional => 'Script (facultatif)';

  @override
  String get modeNone => '— aucun —';

  @override
  String get modePickProxy =>
      'Choisissez un proxy ou un groupe à associer à ce mode.';

  @override
  String get commonClose => 'Fermer';

  @override
  String get commonCancel => 'Annuler';

  @override
  String get commonSave => 'Enregistrer';

  @override
  String get commonSavingEllipsis => 'Enregistrement…';

  @override
  String get commonLoadingEllipsis => 'Chargement...';

  @override
  String get commonApplyingEllipsis => 'Application…';

  @override
  String get trafficUp => 'Envoi';

  @override
  String get trafficDown => 'Réception';

  @override
  String get trafficPeak => 'Pic';

  @override
  String get trafficTotal => 'Total';

  @override
  String get trafficAvg1Min => 'Moy. (1 min)';

  @override
  String get windowKeepOnTop => 'Garder au-dessus';

  @override
  String get windowUnpinFromTop => 'Détacher du dessus';

  @override
  String get windowMinimize => 'Réduire';

  @override
  String get windowRestore => 'Restaurer';

  @override
  String get windowMaximize => 'Agrandir';

  @override
  String get windowClose => 'Fermer';

  @override
  String get settingsAutostart => 'Démarrage automatique';

  @override
  String get settingsAutostartLoading =>
      'Chargement de l’état du démarrage automatique...';

  @override
  String get settingsEnableAutostart => 'Activer le démarrage automatique';

  @override
  String get settingsDisableAutostart => 'Désactiver le démarrage automatique';

  @override
  String get settingsLanguage => 'Langue';

  @override
  String get settingsLanguageHint =>
      'La préférence de langue est enregistrée localement.';

  @override
  String get settingsAppLanguage => 'Langue de l’application';

  @override
  String get settingsTheme => 'Thème';

  @override
  String get settingsThemeHint =>
      'Le mode et la palette du thème sont enregistrés localement.';

  @override
  String get settingsThemeMode => 'Mode de thème';

  @override
  String get settingsThemeFollowSystem => 'Suivre le système';

  @override
  String get settingsThemeLight => 'Clair';

  @override
  String get settingsThemeDark => 'Sombre';

  @override
  String get settingsThemePalette => 'Palette du thème';

  @override
  String get settingsLayout => 'Disposition';

  @override
  String get settingsLayoutHint =>
      'Déplace les icônes des puces à droite lors de la lecture de droite à gauche (arabe, hébreu, ...).';

  @override
  String get settingsChipIconSide => 'Côté de l’icône';

  @override
  String get settingsChipIconLeft => 'Gauche (par défaut)';

  @override
  String get settingsChipIconRight => 'Droite (RTL)';

  @override
  String get networkLocalProxyListenAddress =>
      'Adresse d’écoute du proxy local';

  @override
  String get networkListenHost => 'Hôte d’écoute';

  @override
  String get networkListenPort => 'Port d’écoute';

  @override
  String get networkSystemProxy => 'Proxy système';

  @override
  String get networkEnableSystemProxy => 'Activer le proxy système';

  @override
  String get networkDisableSystemProxy => 'Désactiver le proxy système';

  @override
  String get networkTunSetup => 'Configuration TUN';

  @override
  String get networkTunStatusAvailable => 'L’état TUN est disponible.';

  @override
  String get networkPrepareTunInterface => 'Préparer l’interface TUN';

  @override
  String get networkRemovePreparedInterface => 'Supprimer l’interface préparée';

  @override
  String get networkMixedProtocolToggles =>
      'Interrupteurs de protocoles mixtes';

  @override
  String get networkEnableSocks5Listener => 'Activer l’écouteur SOCKS5';

  @override
  String get networkEnableSocks5Subtitle =>
      'Accepter les clients SOCKS5 locaux sur le port mixte.';

  @override
  String get networkEnableHttpProxyListener =>
      'Activer l’écouteur de proxy HTTP';

  @override
  String get networkEnableHttpProxySubtitle =>
      'Accepter HTTP CONNECT et les requêtes proxy au format absolu.';

  @override
  String get dnsResolverBackend => 'Backend de résolution';

  @override
  String get dnsApplyImmediately =>
      'S’applique immédiatement à l’exécution active.';

  @override
  String get dnsApplyOnNextStart =>
      'Enregistré dans le brouillon courant et utilisé au prochain démarrage.';

  @override
  String get dnsBackend => 'Backend';

  @override
  String get dnsUdpServer => 'Serveur UDP';

  @override
  String get dnsDohUrl => 'URL DoH';

  @override
  String get dnsHelperSystem =>
      'Utilise le résolveur du système hôte pour les décisions de routage par IP.';

  @override
  String get dnsHelperUdp =>
      'Interroge un serveur DNS directement. udp://1.1.1.1:53 et 1.1.1.1:53 sont acceptés.';

  @override
  String get dnsHelperDoh =>
      'Utilise DNS sur HTTPS pour les décisions de routage par IP.';

  @override
  String get dnsApplyDnsSettings => 'Appliquer les réglages DNS';

  @override
  String get advancedDiagnostics => 'Diagnostics';

  @override
  String get advancedRefreshStatus => 'Actualiser l’état';

  @override
  String get advancedValidateConfig => 'Valider la configuration';

  @override
  String get advancedLogLevel => 'Niveau de journal';

  @override
  String get advancedLogLevelHint =>
      'Filtre uniquement ce que la page Journaux affiche. Ne modifie pas encore le niveau d’émission de trace natif.';

  @override
  String get advancedLogsPageFilter => 'Filtre de la page Journaux';

  @override
  String advancedCurrentFilter(String label) {
    return 'Filtre actuel : $label';
  }

  @override
  String get advancedRawConfigEditor => 'Éditeur de configuration brute';

  @override
  String get advancedRawConfigEditorHint =>
      'Modifie le brouillon courant en JSON. TOML est pris en charge pour l’export ; le chargement accepte tout ce que le client natif peut analyser.';

  @override
  String get advancedConfigFilePath => 'Chemin du fichier de configuration';

  @override
  String get advancedLoadFile => 'Charger le fichier';

  @override
  String get advancedExportJson => 'Exporter en JSON';

  @override
  String get advancedExportToml => 'Exporter en TOML';

  @override
  String get advancedLoadCurrentDraft => 'Charger le brouillon courant';

  @override
  String get advancedRawClientConfigJson => 'Config client brute (JSON)';

  @override
  String get advancedApplyJsonToDraft => 'Appliquer le JSON au brouillon';

  @override
  String get proxiesEmptyStopped =>
      'Démarrez le proxy pour inspecter les endpoints et les groupes.';

  @override
  String get proxiesEmptyNoEndpoints =>
      'Aucun endpoint n’a encore été signalé. Actualisez une fois le proxy entièrement démarré.';

  @override
  String get proxiesNoActiveSelection => 'Aucune sélection active';

  @override
  String proxiesActiveLabel(String kind, String name) {
    return '$kind actif : $name';
  }

  @override
  String get proxiesEndpoints => 'Endpoints';

  @override
  String get proxiesAuto => 'auto';

  @override
  String get proxiesUnknownEndpoint => 'endpoint inconnu';

  @override
  String get connectionsCloseAll => 'Tout fermer';

  @override
  String get connectionsEmpty =>
      'Aucune connexion active. Les entrées en direct apparaissent ici tant que le trafic passe par le proxy local.';

  @override
  String connectionsVia(String app) {
    return 'via $app';
  }

  @override
  String get requestsEmpty =>
      'Aucune requête capturée pour l’instant. Envoyez du trafic via le proxy local et les requêtes récentes apparaîtront ici.';

  @override
  String get logsNoMatch =>
      'Aucune entrée ne correspond au filtre de niveau actuel.';

  @override
  String get logsEmpty =>
      'Aucune entrée de journal pour le moment. Les événements récents s’afficheront ici pendant que le proxy est actif.';

  @override
  String get profilesCurrentDraft => 'Brouillon courant';

  @override
  String get profilesProfileName => 'Nom du profil';

  @override
  String get profilesWrongsvImport => 'Import wrongsv';

  @override
  String get profilesWrongsvConfigPath => 'Chemin de configuration wrongsv';

  @override
  String get profilesServerHost => 'Hôte serveur pour la configuration adaptée';

  @override
  String get profilesLocalListenHost => 'Hôte d’écoute local';

  @override
  String get profilesLocalListenPort => 'Port d’écoute local';

  @override
  String get profilesInspectWrongsv => 'Inspecter wrongsv';

  @override
  String get profilesAdaptWrongsv => 'Adapter wrongsv';

  @override
  String get profilesCompleteImport => 'Terminer l’import';

  @override
  String get profilesSavedProfiles => 'Profils enregistrés';

  @override
  String get profilesSavedEmpty =>
      'Aucun profil enregistré. Enregistrez le brouillon courant pour créer une entrée réutilisable.';

  @override
  String get profilesLoadSelected => 'Charger la sélection';

  @override
  String get profilesDuplicateSelected => 'Dupliquer la sélection';

  @override
  String get profilesDeleteSelected => 'Supprimer la sélection';

  @override
  String get profilesNew => 'Nouveau';

  @override
  String get profilesSaveCurrent => 'Enregistrer le courant';

  @override
  String get profilesDeleteTitle => 'Supprimer le profil enregistré ?';

  @override
  String profilesDeleteMessage(String name) {
    return 'Supprimer « $name » de la liste locale ? Le serveur wrongsv distant n’est pas modifié.';
  }

  @override
  String get profilesDeleteConfirm => 'Supprimer le profil';
}
