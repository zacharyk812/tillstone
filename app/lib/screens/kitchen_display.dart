import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

class _Ticket {
  const _Ticket(this.number, this.source, this.minutes, this.items, {this.mode = 'For here', this.recalled = false});

  final String number;
  final String source;
  final int minutes;
  final List<(int, String, List<String>, bool)> items; // qty, name, mods, done
  final String mode;
  final bool recalled;
}

/// Kitchen display for one station: tickets left to right, oldest first,
/// timers that turn amber then red. Tap a ticket to bump it.
class KitchenDisplay extends StatelessWidget {
  const KitchenDisplay({super.key});

  static const _tickets = [
    _Ticket('Table 6', 'Sam · Dining', 14, [
      (1, 'Ribeye', ['Medium rare', 'No butter'], true),
      (1, 'Halibut', ['Allergy: shellfish'], false),
      (2, 'Fries', [], false),
    ], mode: 'Course 2'),
    _Ticket('A39', 'Front register', 9, [
      (1, 'Breakfast sandwich', ['No egg', 'Add avocado'], false),
      (1, 'Avocado toast', [], false),
    ], mode: 'To go'),
    _Ticket('Table 4', 'Sam · Dining', 6, [
      (2, 'Burrata', [], true),
      (1, 'Caesar', ['Dressing on side'], false),
    ], mode: 'Course 1'),
    _Ticket('A41', 'Front register', 2, [
      (1, 'Grilled cheese', ['Kids'], false),
    ]),
    _Ticket(
      'B12',
      'Handheld · Patio',
      1,
      [
        (3, 'Fish tacos', [], false),
        (1, 'Nachos', ['Extra jalapeños'], false),
      ],
      mode: 'For here',
      recalled: true,
    ),
  ];

  @override
  Widget build(BuildContext context) {
    // Kitchens are bright and screens are far away: dark theme, big text.
    return Theme(
      data: PosTheme.dark(),
      child: Builder(
        builder: (context) {
          final c = PosThemeColors.of(context);
          return Scaffold(
            body: SafeArea(
              child: Column(
                children: [
                  Container(
                    height: 56,
                    padding: const EdgeInsets.symmetric(horizontal: PosSpacing.lg),
                    color: c.surface,
                    child: Row(
                      children: [
                        const PosBrand(),
                        const SizedBox(width: PosSpacing.lg),
                        const PosText('Grill & hot line', style: PosType.title),
                        const Spacer(),
                        PosText('5 open · avg 6 min', color: c.textMuted),
                        const SizedBox(width: PosSpacing.lg),
                        PosButton(label: 'Recall', kind: PosButtonKind.secondary, onPressed: () {}),
                        const SizedBox(width: PosSpacing.lg),
                        const PosSyncIndicator(state: PosSyncState.synced, label: 'Connected'),
                      ],
                    ),
                  ),
                  Expanded(
                    child: Padding(
                      padding: const EdgeInsets.all(PosSpacing.md),
                      child: Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          for (final t in _tickets)
                            Expanded(
                              child: Padding(
                                padding: const EdgeInsets.all(PosSpacing.xs),
                                child: _TicketCard(ticket: t),
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
      ),
    );
  }
}

class _TicketCard extends StatelessWidget {
  const _TicketCard({required this.ticket});

  final _Ticket ticket;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    final (fg, _) =
        (ticket.minutes >= 12
                ? PosTone.danger
                : ticket.minutes >= 8
                ? PosTone.warning
                : PosTone.success)
            .colors(c);
    return Container(
      decoration: BoxDecoration(
        color: c.surface,
        borderRadius: BorderRadius.circular(PosRadius.md),
        border: Border.all(color: fg.withValues(alpha: 0.7), width: 2),
      ),
      clipBehavior: Clip.antiAlias,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Container(
            color: fg.withValues(alpha: 0.18),
            padding: const EdgeInsets.all(PosSpacing.md),
            child: Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      PosText(ticket.number, style: PosType.title, weight: FontWeight.w700),
                      PosText(
                        '${ticket.source} · ${ticket.mode}',
                        style: PosType.caption,
                        color: c.textMuted,
                        maxLines: 1,
                      ),
                    ],
                  ),
                ),
                PosText('${ticket.minutes}:00', style: PosType.title, color: fg, weight: FontWeight.w700),
              ],
            ),
          ),
          if (ticket.recalled)
            Container(
              color: c.warning.withValues(alpha: 0.2),
              padding: const EdgeInsets.symmetric(horizontal: PosSpacing.md, vertical: 4),
              child: PosText('Recalled', style: PosType.caption, color: c.warning, weight: FontWeight.w700),
            ),
          for (final (qty, name, mods, done) in ticket.items)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: PosSpacing.md, vertical: PosSpacing.sm + 2),
              decoration: BoxDecoration(
                border: Border(bottom: BorderSide(color: c.surfaceMuted)),
              ),
              child: Opacity(
                opacity: done ? 0.45 : 1,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        PosText('$qty×', style: PosType.label, weight: FontWeight.w700),
                        const SizedBox(width: PosSpacing.sm),
                        Expanded(child: PosText(name, style: PosType.label, size: 17)),
                        if (done) Icon(Icons.check, size: 18, color: c.success),
                      ],
                    ),
                    for (final m in mods)
                      Padding(
                        padding: const EdgeInsets.only(left: 30, top: 2),
                        child: PosText(
                          m,
                          style: PosType.caption,
                          color: m.startsWith('Allergy') ? c.danger : c.warning,
                          weight: FontWeight.w600,
                        ),
                      ),
                  ],
                ),
              ),
            ),
          const Spacer(),
          Padding(
            padding: const EdgeInsets.all(PosSpacing.sm),
            child: PosButton(label: 'Bump', onPressed: () {}),
          ),
        ],
      ),
    );
  }
}
