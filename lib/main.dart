import 'package:flutter/material.dart';

import 'app.dart';
import 'desktop_shell_controller.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final desktopShellController = TrayDesktopShellController();
  await desktopShellController.bootstrap();
  runApp(WrongclApp(desktopShellController: desktopShellController));
}
