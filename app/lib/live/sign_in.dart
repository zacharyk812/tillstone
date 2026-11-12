import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

import '../src/session.dart';
import '../src/vault.dart';
import 'register.dart';

/// Staff PIN sign-in, wired to the register. PINs are checked in Rust
/// (Argon2id), off the UI thread, with lockout after repeated failures.
class SignInScreen extends StatefulWidget {
  const SignInScreen({super.key, required this.session});

  final Session session;

  @override
  State<SignInScreen> createState() => _SignInScreenState();
}

class _SignInScreenState extends State<SignInScreen> {
  String _pin = '';
  String? _message;
  bool _checking = false;

  Future<void> _key(String k) async {
    if (_checking) return;
    setState(() {
      _message = null;
      if (k == 'back') {
        if (_pin.isNotEmpty) _pin = _pin.substring(0, _pin.length - 1);
      } else if (k == 'Clear') {
        _pin = '';
      } else if (_pin.length < 4) {
        _pin += k;
      }
    });
    if (_pin.length == 4) {
      setState(() => _checking = true);
      final ok = await widget.session.signIn(_pin);
      if (!mounted) return;
      setState(() {
        _checking = false;
        _message = ok ? null : (widget.session.error ?? 'Wrong PIN');
        _pin = '';
      });
      if (ok) {
        await Navigator.of(context)
            .push(MaterialPageRoute<void>(builder: (_) => RegisterScreen(session: widget.session)));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    final till = widget.session.till;
    return Scaffold(
      body: SafeArea(
        child: Row(
          children: [
            Expanded(
              child: Container(
                color: c.primary,
                padding: const EdgeInsets.all(PosSpacing.xxl),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Theme(data: PosTheme.dark(), child: const PosBrand()),
                    const Spacer(),
                    PosText(till.businessName(), style: PosType.display, color: c.onPrimary),
                    const SizedBox(height: PosSpacing.sm),
                    PosText(
                      'This device · works offline',
                      style: PosType.title,
                      color: c.onPrimary.withValues(alpha: 0.8),
                    ),
                    if (widget.session.keyStorage == KeyStorage.file) ...[
                      const SizedBox(height: PosSpacing.lg),
                      PosText(
                        'No system keystore found: keys are stored in a protected file.',
                        style: PosType.caption,
                        color: c.onPrimary,
                        weight: FontWeight.w600,
                      ),
                    ],
                    const Spacer(),
                    if (widget.session.state.demo) ...[
                      PosText('Demo PINs', style: PosType.label, color: c.onPrimary),
                      for (final (who, pin) in till.demoPins())
                        PosText('$who: $pin', style: PosType.caption, color: c.onPrimary.withValues(alpha: 0.85)),
                    ],
                  ],
                ),
              ),
            ),
            Expanded(
              child: Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 360),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      PosText(_checking ? 'Checking…' : 'Enter your PIN', style: PosType.title),
                      const SizedBox(height: PosSpacing.xl),
                      PosPinDots(length: 4, filled: _pin.length, error: _message != null),
                      const SizedBox(height: PosSpacing.md),
                      SizedBox(
                        height: 20,
                        child: _message == null
                            ? null
                            : PosText(_message!, style: PosType.caption, color: c.danger, weight: FontWeight.w600),
                      ),
                      const SizedBox(height: PosSpacing.lg),
                      PosNumpad(bottomLeft: 'Clear', onKey: _key),
                    ],
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
