import 'package:flutter_test/flutter_test.dart';
import 'package:wrongcl/app.dart';
import 'package:wrongcl/wrongcl_client.dart';

void main() {
  testWidgets('client controls render and start proxy', (tester) async {
    final client = FakeWrongclClient();

    await tester.pumpWidget(WrongclApp(client: client));
    await tester.pumpAndSettle();

    expect(find.text('Wrongcl'), findsOneWidget);
    expect(find.text('Start proxy'), findsOneWidget);
    expect(find.text('Run probe'), findsOneWidget);

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
      data: {'version': 'test'},
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
      },
    );
  }

  @override
  NativeResponse stopProxy() {
    return const NativeResponse(
      ok: true,
      message: 'SOCKS5 proxy stopped',
      data: {'running': false},
    );
  }

  @override
  NativeResponse status() {
    return const NativeResponse(
      ok: true,
      message: 'SOCKS5 proxy is stopped',
      data: {'running': false},
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
