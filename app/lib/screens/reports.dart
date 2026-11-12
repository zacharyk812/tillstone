import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

/// Manage → Reports: today's sales by selling area, tenders, taxes, and the
/// loss-prevention summary (PLAN §9).
class ReportsScreen extends StatelessWidget {
  const ReportsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    Widget stat(String label, String value, {String? note, PosTone? tone}) => Expanded(
      child: PosSurface(
        padding: const EdgeInsets.all(PosSpacing.lg),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            PosText(label, color: c.textMuted),
            const SizedBox(height: PosSpacing.xs),
            PosText(value, style: PosType.display, size: 28),
            if (note != null) ...[
              const SizedBox(height: PosSpacing.xs),
              PosText(note, style: PosType.caption, color: tone?.colors(c).$1 ?? c.textMuted, weight: FontWeight.w600),
            ],
          ],
        ),
      ),
    );

    Widget bars(String title, List<(String, double, String)> rows) => Expanded(
      child: PosSurface(
        padding: const EdgeInsets.all(PosSpacing.lg),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            PosText(title, style: PosType.label),
            const SizedBox(height: PosSpacing.md),
            for (final (label, share, value) in rows)
              Padding(
                padding: const EdgeInsets.only(bottom: PosSpacing.md),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Expanded(child: PosText(label)),
                        PosText(value, weight: FontWeight.w600),
                      ],
                    ),
                    const SizedBox(height: 6),
                    ClipRRect(
                      borderRadius: BorderRadius.circular(4),
                      child: LinearProgressIndicator(
                        value: share,
                        minHeight: 8,
                        backgroundColor: c.surfaceMuted,
                        color: c.primary,
                      ),
                    ),
                  ],
                ),
              ),
          ],
        ),
      ),
    );

    return Scaffold(
      body: SafeArea(
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Container(
              width: 240,
              decoration: BoxDecoration(
                color: c.surface,
                border: Border(right: BorderSide(color: c.border)),
              ),
              padding: const EdgeInsets.all(PosSpacing.lg),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const PosBrand(),
                  const SizedBox(height: PosSpacing.xl),
                  for (final (icon, label, on) in const [
                    (Icons.insights_outlined, 'Reports', true),
                    (Icons.restaurant_menu, 'Menu & items', false),
                    (Icons.people_outline, 'Staff & roles', false),
                    (Icons.percent, 'Taxes', false),
                    (Icons.storefront_outlined, 'Selling areas', false),
                    (Icons.print_outlined, 'Printers & routes', false),
                    (Icons.devices_other, 'Devices', false),
                    (Icons.backup_outlined, 'Backups', false),
                    (Icons.flag_outlined, 'Review list', false),
                  ])
                    Container(
                      margin: const EdgeInsets.only(bottom: 2),
                      padding: const EdgeInsets.symmetric(horizontal: PosSpacing.md, vertical: PosSpacing.md),
                      decoration: BoxDecoration(
                        color: on ? c.primary.withValues(alpha: 0.1) : null,
                        borderRadius: BorderRadius.circular(PosRadius.md),
                      ),
                      child: Row(
                        children: [
                          Icon(icon, size: 20, color: on ? c.primary : c.textMuted),
                          const SizedBox(width: PosSpacing.md),
                          PosText(label, style: PosType.label, color: on ? c.primary : c.text),
                          if (label == 'Review list') ...[
                            const Spacer(),
                            const PosBadge(label: '2', tone: PosTone.warning),
                          ],
                        ],
                      ),
                    ),
                ],
              ),
            ),
            Expanded(
              child: Padding(
                padding: const EdgeInsets.all(PosSpacing.xl),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Row(
                      children: [
                        const PosText('Today', style: PosType.display),
                        const SizedBox(width: PosSpacing.md),
                        PosText('Wed, Nov 12 · Main Street', color: c.textMuted),
                        const Spacer(),
                        PosSegmented(options: const ['Today', 'Week', 'Month'], selected: 0, onSelected: (_) {}),
                        const SizedBox(width: PosSpacing.md),
                        PosButton(label: 'Z report', kind: PosButtonKind.secondary, onPressed: () {}),
                      ],
                    ),
                    const SizedBox(height: PosSpacing.xl),
                    IntrinsicHeight(
                      child: Row(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          stat('Net sales', r'$4,812.40', note: '+12% vs last Wednesday', tone: PosTone.success),
                          const SizedBox(width: PosSpacing.md),
                          stat('Orders', '286', note: r'Avg $16.83'),
                          const SizedBox(width: PosSpacing.md),
                          stat('Tips', r'$612.25'),
                          const SizedBox(width: PosSpacing.md),
                          stat('Refunds', r'$48.20', note: '3 refunds · all approved', tone: PosTone.neutral),
                        ],
                      ),
                    ),
                    const SizedBox(height: PosSpacing.md),
                    Expanded(
                      child: Row(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          bars('By selling area', const [
                            ('Café', 0.58, r'$2,790.10'),
                            ('Dining room', 0.31, r'$1,492.00'),
                            ('Gift shop', 0.07, r'$336.30'),
                            ('Books', 0.04, r'$194.00'),
                          ]),
                          const SizedBox(width: PosSpacing.md),
                          bars('By tender', const [
                            ('Card', 0.81, r'$3,898.04'),
                            ('Cash', 0.17, r'$818.11'),
                            ('Gift card', 0.02, r'$96.25'),
                          ]),
                          const SizedBox(width: PosSpacing.md),
                          Expanded(
                            child: PosSurface(
                              padding: const EdgeInsets.all(PosSpacing.lg),
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  const PosText('Loss prevention', style: PosType.label),
                                  const SizedBox(height: PosSpacing.md),
                                  for (final (who, what, tone) in const [
                                    ('Maya', '0 voids · 2 discounts', PosTone.success),
                                    ('Sam', '1 void · 1 refund', PosTone.success),
                                    ('Jo', '4 no-sale drawer opens', PosTone.warning),
                                    ('Ari', r'Cash short $12.00', PosTone.danger),
                                  ])
                                    Padding(
                                      padding: const EdgeInsets.only(bottom: PosSpacing.md),
                                      child: Row(
                                        children: [
                                          Expanded(child: PosText(who, weight: FontWeight.w600)),
                                          PosStatusPill(label: what, tone: tone),
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
