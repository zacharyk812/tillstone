// M3: a complete cash sale through the real app, on a real platform:
// create the demo business, sign in by PIN, ring up an espresso, pay cash.
//
//   flutter test integration_test/sale_test.dart -d macos

import 'dart:io';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:material_ui/material_ui.dart' show ValueKey;
import 'package:tillstone/live/start.dart';
import 'package:tillstone/main.dart';
import 'package:tillstone/src/rust/frb_generated.dart';
import 'package:tillstone/src/vault.dart';

/// A keystore that's never available, so the test never touches the real
/// one and exercises the file fallback.
class _NoKeystore extends FlutterSecureStorage {
  const _NoKeystore();

  @override
  Future<String?> read({
    required String key,
    AppleOptions? iOptions,
    AndroidOptions? aOptions,
    LinuxOptions? lOptions,
    WebOptions? webOptions,
    AppleOptions? mOptions,
    WindowsOptions? wOptions,
  }) => throw StateError('no keystore in tests');
}

Future<void> _waitFor(WidgetTester tester, Finder finder) async {
  for (var i = 0; i < 200 && finder.evaluate().isEmpty; i++) {
    await tester.pump(const Duration(milliseconds: 100));
  }
  expect(finder, findsWidgets);
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('a cash sale from start to finish', (tester) async {
    final dir = Directory.systemTemp.createTempSync('tillstone-sale-');
    await tester.pumpWidget(
      TillstoneApp(
        home: StartGate(
          vault: KeyVault(storage: const _NoKeystore(), directory: dir),
        ),
      ),
    );

    await _waitFor(tester, find.text('Try the demo business'));
    await tester.tap(find.text('Try the demo business'));
    await _waitFor(tester, find.text('Enter your PIN'));
    expect(find.textContaining('protected file'), findsOneWidget, reason: 'fallback storage is disclosed');

    for (final d in '1111'.split('')) {
      await tester.tap(find.text(d).last);
      await tester.pump();
    }
    await _waitFor(tester, find.text('Maya'));

    // Switch to the café (selling areas are listed by name: Books, Café).
    await tester.tap(find.text('Café'));
    await _waitFor(tester, find.byKey(const ValueKey('item-Espresso')));

    await tester.tap(find.byKey(const ValueKey('item-Espresso')));
    // Espresso has optional extras: the options dialog opens; add it plain.
    await _waitFor(tester, find.text('Add'));
    await tester.tap(find.text('Add'));
    await _waitFor(tester, find.text(r'Pay $3.25'));
    expect(find.text('Order A1'), findsOneWidget);

    await tester.tap(find.text(r'Pay $3.25'));
    await _waitFor(tester, find.byKey(const ValueKey('quick-500')));
    await tester.tap(find.byKey(const ValueKey('quick-500')));
    await _waitFor(tester, find.text('Paid'));
    expect(find.text(r'$1.75'), findsOneWidget);

    await tester.tap(find.text('New order'));
    await _waitFor(tester, find.text('Tap an item to start'));
    dir.deleteSync(recursive: true);
  });
}
