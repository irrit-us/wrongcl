import 'package:flutter_test/flutter_test.dart';
import 'package:wrongcl/app.dart';
import 'package:wrongcl/wrongcl_client.dart';

void main() {
  testWidgets('client controls render and start proxy', (tester) async {
    final client = FakeWrongclClient();

    await tester.pumpWidget(WrongclApp(client: client));
    await tester.pumpAndSettle();

    expect(find.text('Wrongcl'), findsOneWidget);
    expect(find.text('Connection Manager'), findsOneWidget);
    expect(find.text('Start proxy'), findsOneWidget);
    expect(find.text('Run probe'), findsOneWidget);

    await tester.ensureVisible(find.text('Start proxy'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Start proxy'));
    await tester.pumpAndSettle();

    expect(client.started, isTrue);
    expect(find.textContaining('SOCKS5 proxy started'), findsOneWidget);
  });
}

class FakeWrongclClient implements WrongclClient {
  bool started = false;

  @override
  NativeResponse version() {
    return const NativeResponse(
      ok: true,
      message: 'native ready',
      data: {
        'version': 'test',
        'protocols': ['raw-vless-tcp'],
      },
    );
  }

  @override
  NativeResponse startProxy(ClientSettings settings) {
    started = true;
    return NativeResponse(
      ok: true,
      message: 'SOCKS5 proxy started',
      data: {
        'running': true,
        'local_host': settings.localHost,
        'local_port': settings.localPort,
        'active_connections': 0,
        'total_connections': 0,
        'failed_connections': 0,
        'bytes_uploaded': 0,
        'bytes_downloaded': 0,
      },
    );
  }

  @override
  NativeResponse stopProxy() {
    return const NativeResponse(
      ok: true,
      message: 'SOCKS5 proxy stopped',
      data: {
        'running': false,
        'active_connections': 0,
        'total_connections': 0,
        'failed_connections': 0,
        'bytes_uploaded': 0,
        'bytes_downloaded': 0,
      },
    );
  }

  @override
  NativeResponse status() {
    return const NativeResponse(
      ok: true,
      message: 'SOCKS5 proxy is stopped',
      data: {
        'running': false,
        'active_connections': 0,
        'total_connections': 0,
        'failed_connections': 0,
        'bytes_uploaded': 0,
        'bytes_downloaded': 0,
      },
    );
  }

  @override
  NativeResponse probe(ClientSettings settings) {
    return const NativeResponse(
      ok: true,
      message: 'probe succeeded',
      data: {'bytes_read': 4, 'preview': 'pong'},
    );
  }
}
