import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

import '../screens/gallery.dart';
import '../src/session.dart';
import '../src/vault.dart';
import 'recovery_kit.dart';
import 'setup.dart';
import 'sign_in.dart';

/// First screen: opens this device's register, or offers to create one.
class StartGate extends StatefulWidget {
  const StartGate({super.key, this.vault});

  final KeyVault? vault;

  @override
  State<StartGate> createState() => _StartGateState();
}

class _StartGateState extends State<StartGate> {
  late final KeyVault _vault = widget.vault ?? KeyVault();
  late Future<Session?> _session = Session.tryOpen(_vault);
  bool _creating = false;

  Future<void> _createDemo() async {
    setState(() => _creating = true);
    try {
      final session = await Session.createDemo(_vault);
      if (!mounted) return;
      setState(() {
        _session = Future.value(session);
      });
    } on Object catch (e) {
      if (!mounted) return;
      setState(() => _creating = false);
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Could not create the demo: $e')));
    }
  }

  void _setup() {
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => SetupWizard(
          vault: _vault,
          onCreated: (session) => setState(() {
            _session = Future.value(session);
          }),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<Session?>(
      future: _session,
      builder: (context, snap) {
        if (snap.connectionState != ConnectionState.done) {
          return const Scaffold(body: Center(child: CircularProgressIndicator()));
        }
        if (snap.hasError) {
          return _Message(title: 'Tillstone could not start', body: '${snap.error}');
        }
        final session = snap.data;
        if (session != null) {
          // Setup isn't finished until the recovery code is saved.
          return ListenableBuilder(
            listenable: session,
            builder: (context, _) =>
                session.state.recoveryConfirmed ? SignInScreen(session: session) : RecoveryKitScreen(session: session),
          );
        }
        return _Welcome(creating: _creating, onDemo: _createDemo, onSetup: _setup);
      },
    );
  }
}

class _Welcome extends StatelessWidget {
  const _Welcome({required this.creating, required this.onDemo, required this.onSetup});

  final bool creating;
  final VoidCallback onDemo;
  final VoidCallback onSetup;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return Scaffold(
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 520),
          child: Padding(
            padding: const EdgeInsets.all(PosSpacing.xl),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const Align(alignment: Alignment.centerLeft, child: PosBrand()),
                const SizedBox(height: PosSpacing.xl),
                const PosText('Welcome', style: PosType.display),
                const SizedBox(height: PosSpacing.sm),
                PosText(
                  '${PosBrandInfo.name} runs entirely on this device: no account, no cloud. '
                  'Try the demo business first, or set up your own in a few minutes.',
                  color: c.textMuted,
                ),
                const SizedBox(height: PosSpacing.xl),
                PosButton(
                  label: creating ? 'Creating…' : 'Try the demo business',
                  large: true,
                  onPressed: creating ? null : onDemo,
                ),
                const SizedBox(height: PosSpacing.sm),
                PosButton(
                  label: 'Set up my business',
                  kind: PosButtonKind.secondary,
                  large: true,
                  onPressed: creating ? null : onSetup,
                ),
                const SizedBox(height: PosSpacing.sm),
                PosButton(
                  label: 'Look at the screen designs',
                  kind: PosButtonKind.secondary,
                  onPressed: () =>
                      Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => const ScreenGallery())),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _Message extends StatelessWidget {
  const _Message({required this.title, required this.body});

  final String title;
  final String body;

  @override
  Widget build(BuildContext context) => Scaffold(
    body: Center(
      child: Padding(
        padding: const EdgeInsets.all(PosSpacing.xl),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            PosText(title, style: PosType.title),
            const SizedBox(height: PosSpacing.sm),
            PosText(body, align: TextAlign.center),
          ],
        ),
      ),
    ),
  );
}
