// M3: setting up a real business through the wizard, on a real platform:
// business, money and tax, owner, passphrase, recovery kit, then sign in.
//
//   flutter test integration_test/setup_test.dart -d macos

import 'dart:io';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:material_ui/material_ui.dart' show SelectableText, SizedBox, Text, TextField, ValueKey, Widget;
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
  if (finder.evaluate().isEmpty) {
    final texts = find.byType(Text).evaluate().map((e) => (e.widget as Text).data).whereType<String>();
    // ignore: avoid_print
    print('On screen: ${texts.join(' | ')}');
  }
  expect(finder, findsWidgets);
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('set up a business, save the recovery code, sign in', (tester) async {
    final dir = Directory.systemTemp.createTempSync('tillstone-setup-');
    Widget app() => TillstoneApp(
      home: StartGate(
        vault: KeyVault(storage: const _NoKeystore(), directory: dir),
      ),
    );
    await tester.pumpWidget(app());

    Finder field(String key) => find.descendant(of: find.byKey(ValueKey(key)), matching: find.byType(TextField));
    // Focus first: on a real platform, typing into a field that already
    // had focus needs the focus change to settle.
    Future<void> type(String key, String text) async {
      await tester.tap(field(key));
      await tester.pump();
      await tester.enterText(field(key), text);
      await tester.pump();
    }

    await _waitFor(tester, find.text('Set up my business'));
    await tester.tap(find.text('Set up my business'));
    await _waitFor(tester, find.text('Your business'));

    // Step 1: name and kind. Next without a name is refused.
    await tester.tap(find.text('Next'));
    await tester.pump();
    expect(find.text('Enter your business name'), findsOneWidget);
    await type('setup-name', 'Harbor Coffee');
    await tester.tap(find.byKey(const ValueKey('template-cafe')));
    await tester.tap(find.text('Next'));
    await _waitFor(tester, find.text('Money and tax'));

    // Step 2: tax rate.
    await type('setup-tax-percent', '8.25');
    await tester.tap(find.text('Next'));
    await _waitFor(tester, find.text('You'));

    // Step 3: owner and PIN; mismatched PINs are caught.
    await type('setup-owner', 'Sam Rivera');
    await type('setup-pin', '2468');
    await type('setup-pin2', '2469');
    await tester.tap(find.text('Next'));
    await tester.pump();
    expect(find.text("The PINs don't match"), findsOneWidget);
    await type('setup-pin2', '2468');
    await tester.tap(find.text('Next'));
    await _waitFor(tester, find.text('Owner passphrase'));

    // Step 4: passphrase; too short is refused.
    await type('setup-pass', 'short');
    await tester.tap(find.text('Create business'));
    await tester.pump();
    expect(find.textContaining('at least 10 characters'), findsWidgets);
    await type('setup-pass', 'harbor tide lantern');
    await type('setup-pass2', 'harbor tide lantern');
    await tester.tap(find.text('Create business'));

    // Recovery kit: the code is shown; Finish needs the checkbox.
    await _waitFor(tester, find.byKey(const ValueKey('recovery-code')));
    final code = tester.widget<SelectableText>(find.byKey(const ValueKey('recovery-code'))).data!;
    expect(code, startsWith('TS1 '));

    // Restart before confirming: the app asks for the passphrase and shows
    // the same code again.
    await tester.pumpWidget(const SizedBox());
    await tester.pumpWidget(app());
    await _waitFor(tester, find.text('Show recovery code'));
    await type('recovery-pass', 'wrong passphrase');
    await tester.tap(find.text('Show recovery code'));
    await _waitFor(tester, find.textContaining('wrong owner passphrase'));
    await type('recovery-pass', 'harbor tide lantern');
    await tester.tap(find.text('Show recovery code'));
    await _waitFor(tester, find.byKey(const ValueKey('recovery-code')));
    expect(tester.widget<SelectableText>(find.byKey(const ValueKey('recovery-code'))).data, code);

    await tester.tap(find.byKey(const ValueKey('recovery-saved')));
    await tester.pump();
    await tester.tap(find.text('Finish setup'));
    await _waitFor(tester, find.text('Enter your PIN'));
    expect(find.text('Harbor Coffee'), findsOneWidget);
    expect(find.text('Demo PINs'), findsNothing, reason: 'a real business never shows PINs');

    for (final d in '2468'.split('')) {
      await tester.tap(find.text(d).last);
      await tester.pump();
    }
    await _waitFor(tester, find.text('Sam Rivera'));
    await _waitFor(tester, find.byKey(const ValueKey('item-Latte')));
    expect(find.text(r'$4.75'), findsNothing, reason: 'Latte has sizes, so it shows "from"');
    expect(find.text(r'from $4.25'), findsOneWidget);
    dir.deleteSync(recursive: true);
  });
}
