import 'package:flutter_test/flutter_test.dart';
import 'package:wrongcl/app.dart';
import 'package:wrongcl/wrongcl_client.dart';

void main() {
  testWidgets('client controls render and start proxy', (tester) async {
    final client = FakeWrongclClient();

    await tester.pumpWidget(WrongclApp(client: client));
    await tester.pumpAndSettle();

    expect(find.text('Wrongcl'), findsOneWidget);
    expect(find.text('Endpoint'), findsOneWidget);
    expect(find.text('Connection Manager'), findsOneWidget);
    expect(find.text('Start proxy'), findsOneWidget);
    expect(find.text('Run probe'), findsOneWidget);

    await tester.ensureVisible(find.text('Start proxy'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Start proxy'));
    await tester.pumpAndSettle();

    expect(client.startCount, 1);
    expect(find.textContaining('SOCKS5 proxy started'), findsOneWidget);

    await tester.ensureVisible(find.text('Run probe'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Run probe'));
    await tester.pumpAndSettle();

    expect(client.probeCount, 1);
    expect(find.textContaining('probe succeeded'), findsOneWidget);
  });
}

class FakeWrongclClient implements WrongclClient {
  int startCount = 0;
  int probeCount = 0;

  @override
  NativeResponse version() {
    return const NativeResponse(
      ok: true,
      message: 'native ready',
      data: {
        'version': 'test',
        'proxies': ['vless', 'trojan', 'mixed', 'shadowsocks'],
        'transports': ['raw', 'websocket', 'httpupgrade'],
        'outer_security': ['none', 'tls'],
      },
    );
  }

  @override
  NativeResponse startProxy(ClientConfigInput config) {
    startCount += 1;
    return NativeResponse(
      ok: true,
      message: 'SOCKS5 proxy started',
      data: {
        'stack': 'VLESS → raw → TCP',
        'proxy': {
          'running': true,
          'local_host': config.localHost,
          'local_port': config.localPort,
          'active_connections': 0,
          'total_connections': 0,
          'failed_connections': 0,
          'bytes_uploaded': 0,
          'bytes_downloaded': 0,
        },
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
  NativeResponse probe(ProbeRequest request) {
    probeCount += 1;
    return const NativeResponse(
      ok: true,
      message: 'probe succeeded',
      data: {
        'stack': 'VLESS → raw → TCP',
        'probe': {'bytes_read': 4, 'preview': 'pong'},
      },
    );
  }

  @override
  NativeResponse stackSummary(ClientConfigInput config) {
    return const NativeResponse(
      ok: true,
      message: 'stack resolved',
      data: {
        'stack': 'VLESS → raw → TCP',
        'proxy': 'vless',
        'transport': 'raw',
        'outer_security': 'none',
      },
    );
  }
}
