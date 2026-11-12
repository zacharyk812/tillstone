import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

/// Staff sign in with a PIN on the in-app keypad. Works offline.
class PinLogin extends StatefulWidget {
  const PinLogin({super.key, this.initialDigits = 0});

  final int initialDigits;

  @override
  State<PinLogin> createState() => _PinLoginState();
}

class _PinLoginState extends State<PinLogin> {
  late int _digits = widget.initialDigits;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
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
                    PosText('Corner Café & Books', style: PosType.display, color: c.onPrimary),
                    const SizedBox(height: PosSpacing.sm),
                    PosText('Front register · Café', style: PosType.title, color: c.onPrimary.withValues(alpha: 0.8)),
                    const SizedBox(height: PosSpacing.xl),
                    Row(
                      children: [
                        Container(
                          width: 9,
                          height: 9,
                          decoration: const BoxDecoration(color: Color(0xFF8FD5A6), shape: BoxShape.circle),
                        ),
                        const SizedBox(width: PosSpacing.sm),
                        PosText('3 devices · all synced · 9:38 AM', color: c.onPrimary.withValues(alpha: 0.8)),
                      ],
                    ),
                    const Spacer(),
                    PosText('Training mode is off', style: PosType.caption, color: c.onPrimary.withValues(alpha: 0.7)),
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
                      const PosText('Enter your PIN', style: PosType.title),
                      const SizedBox(height: PosSpacing.xl),
                      PosPinDots(length: 4, filled: _digits),
                      const SizedBox(height: PosSpacing.xxl),
                      PosNumpad(
                        bottomLeft: 'Clear',
                        onKey: (k) => setState(() {
                          if (k == 'back') {
                            _digits = (_digits - 1).clamp(0, 4);
                          } else if (k == 'Clear') {
                            _digits = 0;
                          } else {
                            _digits = (_digits + 1).clamp(0, 4);
                          }
                        }),
                      ),
                      const SizedBox(height: PosSpacing.lg),
                      PosText('Forgot your PIN? Ask a manager.', style: PosType.caption, color: c.textMuted),
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
