import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

/// Taking payment: choose a tender, enter cash with quick amounts, see the
/// change due. Card payments go to a smart reader from the shared pool.
class PaymentScreen extends StatelessWidget {
  const PaymentScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    Widget tender(IconData icon, String label, {bool on = false, String? note}) => Expanded(
      child: Material(
        color: on ? c.primary.withValues(alpha: 0.08) : c.surface,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(PosRadius.md),
          side: BorderSide(color: on ? c.primary : c.border, width: on ? 2 : 1),
        ),
        child: InkWell(
          onTap: () {},
          borderRadius: BorderRadius.circular(PosRadius.md),
          child: SizedBox(
            height: 104,
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(icon, size: 28, color: on ? c.primary : c.text),
                const SizedBox(height: PosSpacing.sm),
                PosText(label, style: PosType.label, color: on ? c.primary : c.text),
                if (note != null) PosText(note, style: PosType.caption, color: c.textMuted),
              ],
            ),
          ),
        ),
      ),
    );

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
                  PosIconButton(icon: Icons.arrow_back, label: 'Back to order', onPressed: () {}),
                  const SizedBox(width: PosSpacing.md),
                  const PosText('Pay order A42', style: PosType.title),
                  const Spacer(),
                  const PosSyncIndicator(state: PosSyncState.synced, label: 'Front reader ready'),
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
                                const PosText(r'$17.29', style: PosType.display, size: 48),
                                const SizedBox(height: PosSpacing.md),
                                const Row(
                                  children: [
                                    PosStatusPill(label: 'Split: 1 of 1', tone: PosTone.neutral),
                                    SizedBox(width: PosSpacing.sm),
                                    PosStatusPill(label: 'Tip asked on reader', tone: PosTone.primary),
                                  ],
                                ),
                              ],
                            ),
                          ),
                          const SizedBox(height: PosSpacing.lg),
                          Row(
                            children: [
                              tender(Icons.payments_outlined, 'Cash', on: true),
                              const SizedBox(width: PosSpacing.md),
                              tender(Icons.contactless_outlined, 'Card', note: 'Front reader'),
                              const SizedBox(width: PosSpacing.md),
                              tender(Icons.card_giftcard, 'Gift card'),
                              const SizedBox(width: PosSpacing.md),
                              tender(Icons.call_split, 'Split'),
                            ],
                          ),
                          const SizedBox(height: PosSpacing.lg),
                          const PosText('Quick cash', style: PosType.label),
                          const SizedBox(height: PosSpacing.sm),
                          Row(
                            children: [
                              for (final (i, a) in const [r'$17.29', r'$20', r'$40', r'$50', r'$100'].indexed) ...[
                                if (i > 0) const SizedBox(width: PosSpacing.sm),
                                Expanded(
                                  child: PosButton(
                                    label: a,
                                    kind: PosButtonKind.secondary,
                                    large: true,
                                    onPressed: () {},
                                  ),
                                ),
                              ],
                            ],
                          ),
                          const Spacer(),
                          PosSurface(
                            color: c.success.withValues(alpha: 0.08),
                            padding: const EdgeInsets.all(PosSpacing.lg),
                            child: Row(
                              children: [
                                PosText('Change due', style: PosType.title, color: c.success),
                                const Spacer(),
                                PosText(r'$2.71', style: PosType.display, color: c.success),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: PosSpacing.xl),
                    SizedBox(
                      width: 360,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          PosSurface(
                            padding: const EdgeInsets.all(PosSpacing.lg),
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.end,
                              children: [
                                PosText('Cash given', color: c.textMuted),
                                const PosText(r'$20.00', style: PosType.display),
                              ],
                            ),
                          ),
                          const SizedBox(height: PosSpacing.md),
                          PosNumpad(onKey: (_) {}, keyHeight: 72),
                          const SizedBox(height: PosSpacing.md),
                          PosButton(label: 'Take cash · open drawer', large: true, onPressed: () {}),
                        ],
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
  }
}
