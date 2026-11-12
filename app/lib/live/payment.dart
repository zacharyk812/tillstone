import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

import '../src/session.dart';

/// Cash payment, wired to the register. Card payments arrive in M5.
class PaymentScreenLive extends StatefulWidget {
  const PaymentScreenLive({super.key, required this.session});

  final Session session;

  @override
  State<PaymentScreenLive> createState() => _PaymentScreenLiveState();
}

class _PaymentScreenLiveState extends State<PaymentScreenLive> {
  String _digits = '';

  int get _givenMinor => int.tryParse(_digits) ?? 0;

  String _format(int minor) {
    final whole = minor ~/ 100;
    final cents = (minor % 100).toString().padLeft(2, '0');
    return '\$$whole.$cents';
  }

  void _key(String k) => setState(() {
    if (k == 'back') {
      if (_digits.isNotEmpty) _digits = _digits.substring(0, _digits.length - 1);
    } else if (_digits.length < 7) {
      _digits = (_digits + k).replaceFirst(RegExp('^0+'), '');
    }
  });

  /// Common quick-cash amounts: exact, then the next round notes above it.
  List<int> _quick(int due) {
    final out = <int>{due};
    for (final note in [500, 1000, 2000, 5000, 10000]) {
      final next = ((due + note - 1) ~/ note) * note;
      if (next > due) out.add(next);
      if (out.length >= 5) break;
    }
    return out.toList()..sort();
  }

  Future<void> _take(int given) async {
    final result = await widget.session.payCash(given);
    if (!mounted) return;
    if (result == null) {
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(widget.session.error ?? 'Payment failed')));
      return;
    }
    if (!result.closed) {
      setState(() => _digits = '');
      return;
    }
    await showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (context) {
        final c = PosThemeColors.of(context);
        return AlertDialog(
          title: const PosText('Paid', style: PosType.title),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              PosText('Change due', color: c.textMuted),
              PosText(result.change, style: PosType.display, size: 48, color: c.success),
            ],
          ),
          actions: [PosButton(label: 'New order', large: true, onPressed: () => Navigator.of(context).pop())],
        );
      },
    );
    widget.session.newOrder();
    if (mounted) Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return ListenableBuilder(
      listenable: widget.session,
      builder: (context, _) {
        final order = widget.session.order;
        if (order == null) return const Scaffold();
        final due = order.balanceMinor.toInt();
        return Scaffold(
          body: SafeArea(
            child: Column(
              children: [
                Container(
                  height: 60,
                  padding: const EdgeInsets.symmetric(horizontal: PosSpacing.lg),
                  decoration: BoxDecoration(
                    color: c.surface,
                    border: Border(bottom: BorderSide(color: c.border)),
                  ),
                  child: Row(
                    children: [
                      PosIconButton(
                        icon: Icons.arrow_back,
                        label: 'Back to order',
                        onPressed: () => Navigator.of(context).pop(),
                      ),
                      const SizedBox(width: PosSpacing.md),
                      PosText('Pay order ${order.number}', style: PosType.title),
                    ],
                  ),
                ),
                Expanded(
                  child: Padding(
                    padding: const EdgeInsets.all(PosSpacing.xl),
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.stretch,
                            children: [
                              PosSurface(
                                padding: const EdgeInsets.all(PosSpacing.xl),
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    PosText('Amount due', color: c.textMuted),
                                    PosText(order.balance, style: PosType.display, size: 48),
                                    if (order.balanceMinor != order.totalMinor)
                                      PosText('of ${order.total}', color: c.textMuted),
                                  ],
                                ),
                              ),
                              const SizedBox(height: PosSpacing.lg),
                              const PosText('Quick cash', style: PosType.label),
                              const SizedBox(height: PosSpacing.sm),
                              Row(
                                children: [
                                  for (final (i, amount) in _quick(due).indexed) ...[
                                    if (i > 0) const SizedBox(width: PosSpacing.sm),
                                    Expanded(
                                      child: PosButton(
                                        key: ValueKey('quick-$amount'),
                                        label: _format(amount),
                                        kind: PosButtonKind.secondary,
                                        large: true,
                                        onPressed: widget.session.busy ? null : () => _take(amount),
                                      ),
                                    ),
                                  ],
                                ],
                              ),
                              const SizedBox(height: PosSpacing.lg),
                              PosSurface(
                                padding: const EdgeInsets.all(PosSpacing.lg),
                                child: Row(
                                  children: [
                                    Icon(Icons.contactless_outlined, color: c.textMuted),
                                    const SizedBox(width: PosSpacing.sm),
                                    Expanded(
                                      child: PosText('Card payments arrive with milestone M5.', color: c.textMuted),
                                    ),
                                  ],
                                ),
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(width: PosSpacing.xl),
                        SizedBox(
                          width: 360,
                          child: LayoutBuilder(
                            // Keys shrink on short screens, never below a 48px target.
                            builder: (context, box) => Column(
                              crossAxisAlignment: CrossAxisAlignment.stretch,
                              children: [
                                PosSurface(
                                  padding: const EdgeInsets.all(PosSpacing.lg),
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.end,
                                    children: [
                                      PosText('Cash given', color: c.textMuted),
                                      PosText(_format(_givenMinor), style: PosType.display),
                                    ],
                                  ),
                                ),
                                const SizedBox(height: PosSpacing.md),
                                PosNumpad(onKey: _key, keyHeight: ((box.maxHeight - 200) / 4 - 8).clamp(48.0, 72.0)),
                                const SizedBox(height: PosSpacing.md),
                                PosButton(
                                  label: 'Take cash',
                                  large: true,
                                  onPressed: _givenMinor > 0 && !widget.session.busy ? () => _take(_givenMinor) : null,
                                ),
                              ],
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}
