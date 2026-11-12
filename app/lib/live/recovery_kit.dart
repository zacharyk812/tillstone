import 'package:flutter/services.dart' show Clipboard, ClipboardData;
import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

import '../src/session.dart';

/// The recovery kit (PLAN §6): the owner writes down the recovery code
/// before setup is complete. If the app closed before that, the owner
/// enters the passphrase to see the code again.
class RecoveryKitScreen extends StatefulWidget {
  const RecoveryKitScreen({super.key, required this.session});

  final Session session;

  @override
  State<RecoveryKitScreen> createState() => _RecoveryKitScreenState();
}

class _RecoveryKitScreenState extends State<RecoveryKitScreen> {
  late String? _code = widget.session.recoveryCode;
  final _pass = TextEditingController();
  String? _error;
  bool _saved = false;
  bool _busy = false;

  @override
  void dispose() {
    _pass.dispose();
    super.dispose();
  }

  Future<void> _reveal() async {
    setState(() => _busy = true);
    try {
      final code = await widget.session.revealRecoveryCode(_pass.text);
      if (mounted) setState(() => _code = code);
    } on Object catch (e) {
      if (mounted) setState(() => _error = '$e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    final code = _code;
    return Scaffold(
      body: SafeArea(
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 640),
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(PosSpacing.xl),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Align(alignment: Alignment.centerLeft, child: PosBrand()),
                  const SizedBox(height: PosSpacing.xl),
                  const PosText('Save your recovery code', style: PosType.display),
                  const SizedBox(height: PosSpacing.sm),
                  PosText(
                    'If you forget the owner passphrase or lose every owner device, this code is the only '
                    'way to manage ${widget.session.till.businessName()} again. Write it down and keep it '
                    'somewhere safe, away from the register. Anyone with it can act as the owner.',
                    color: c.textMuted,
                  ),
                  const SizedBox(height: PosSpacing.xl),
                  if (code == null) ...[
                    PosTextField(
                      key: const ValueKey('recovery-pass'),
                      label: 'Owner passphrase',
                      help: 'Setup was interrupted. Enter the passphrase to show the code again.',
                      controller: _pass,
                      error: _error,
                      obscure: true,
                      autofocus: true,
                      onSubmitted: (_) => _reveal(),
                    ),
                    const SizedBox(height: PosSpacing.lg),
                    PosButton(label: _busy ? 'Checking…' : 'Show recovery code', onPressed: _busy ? null : _reveal),
                  ] else ...[
                    PosSurface(
                      padding: const EdgeInsets.all(PosSpacing.xl),
                      child: SelectableText(
                        code.replaceAll('-', ' '),
                        key: const ValueKey('recovery-code'),
                        textAlign: TextAlign.center,
                        style: TextStyle(
                          fontFamily: PosTheme.fontFamily,
                          package: PosTheme.fontPackage,
                          fontSize: 22,
                          height: 1.6,
                          letterSpacing: 1.5,
                          fontWeight: FontWeight.w600,
                          fontFeatures: const [FontFeature.tabularFigures()],
                          color: c.text,
                        ),
                      ),
                    ),
                    const SizedBox(height: PosSpacing.sm),
                    Align(
                      alignment: Alignment.centerRight,
                      child: PosButton(
                        label: 'Copy',
                        kind: PosButtonKind.secondary,
                        onPressed: () {
                          Clipboard.setData(ClipboardData(text: code));
                          ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(
                              content: Text('Copied. Paste it somewhere safe, then clear your clipboard.'),
                            ),
                          );
                        },
                      ),
                    ),
                    const SizedBox(height: PosSpacing.lg),
                    MergeSemantics(
                      child: CheckboxListTile(
                        key: const ValueKey('recovery-saved'),
                        contentPadding: EdgeInsets.zero,
                        controlAffinity: ListTileControlAffinity.leading,
                        value: _saved,
                        onChanged: (v) => setState(() => _saved = v ?? false),
                        title: const PosText("I've written down the recovery code and put it somewhere safe"),
                      ),
                    ),
                    const SizedBox(height: PosSpacing.lg),
                    PosButton(
                      label: 'Finish setup',
                      large: true,
                      onPressed: _saved ? widget.session.confirmRecovery : null,
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
