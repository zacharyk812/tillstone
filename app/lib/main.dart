import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

import 'src/background_service.dart';
import 'src/rust/api/core.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  await BackgroundService.start();
  runApp(const TillstoneApp());
}

class TillstoneApp extends StatelessWidget {
  const TillstoneApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Tillstone',
      debugShowCheckedModeBanner: false,
      theme: PosTheme.light(),
      darkTheme: PosTheme.dark(),
      home: const ProofScreen(),
    );
  }
}

/// M0 proof screen: shows that the Rust core runs on this platform.
class ProofScreen extends StatefulWidget {
  const ProofScreen({super.key});

  @override
  State<ProofScreen> createState() => _ProofScreenState();
}

class _ProofScreenState extends State<ProofScreen> {
  String _peerResult = 'Not run yet';
  bool _running = false;
  String? _background;

  @override
  void initState() {
    super.initState();
    if (BackgroundService.supported) {
      BackgroundService.status().then((s) {
        if (mounted && s != null) setState(() => _background = s.toString());
      });
    }
  }

  Future<void> _runPeerTest() async {
    setState(() => _running = true);
    try {
      final r = await peerSelfTest();
      _peerResult = 'Device ${r.sender} → ${r.receiver}: "${r.message}" '
          'in ${r.roundTripMs} ms. '
          'Outsider refused: ${r.outsiderRefused ? 'yes' : 'NO'}';
    } catch (e) {
      _peerResult = 'Failed: $e';
    }
    if (mounted) setState(() => _running = false);
  }

  @override
  Widget build(BuildContext context) {
    final split = splitEvenly(amountMinor: 1000, parts: 3);
    return Scaffold(
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.all(PosSpacing.lg),
          children: [
            Text('Tillstone', style: Theme.of(context).textTheme.headlineMedium),
            const SizedBox(height: PosSpacing.xs),
            Text('Core ${coreVersion()} · M0 proof'),
            const SizedBox(height: PosSpacing.lg),
            Text('Money in Rust: 10.00 USD split 3 ways = ${split.join(' + ')}'),
            const SizedBox(height: PosSpacing.lg),
            PosButton(
              label: _running ? 'Running…' : 'Run peer-to-peer self-test',
              onPressed: _running ? null : _runPeerTest,
            ),
            const SizedBox(height: PosSpacing.md),
            Text(_peerResult, key: const Key('peer-result')),
            if (_background != null) ...[
              const SizedBox(height: PosSpacing.lg),
              Text(_background!, key: const Key('keep-alive')),
            ],
          ],
        ),
      ),
    );
  }
}
