import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

import 'common.dart';

enum TableState { free, seated, ordered, check, overdue }

class _Table {
  const _Table(
    this.name,
    this.x,
    this.y,
    this.w,
    this.h,
    this.state, {
    this.seats = 4,
    this.minutes,
    this.server,
    this.round = false,
    this.total,
  });

  final String name;
  final double x, y, w, h;
  final TableState state;
  final int seats;
  final int? minutes;
  final String? server;
  final bool round;
  final String? total;
}

/// Full-service home: the floor plan, colored by table status, with the
/// selected table's check on the right.
class FloorPlan extends StatelessWidget {
  const FloorPlan({super.key});

  static const _tables = [
    _Table('1', 0.04, 0.08, 0.12, 0.18, TableState.free, seats: 2, round: true),
    _Table('2', 0.20, 0.08, 0.12, 0.18, TableState.seated, seats: 2, round: true, minutes: 6, server: 'Jo'),
    _Table('3', 0.36, 0.08, 0.12, 0.18, TableState.free, seats: 2, round: true),
    _Table('4', 0.04, 0.36, 0.20, 0.20, TableState.ordered, minutes: 24, server: 'Sam', total: r'$86.40'),
    _Table('5', 0.28, 0.36, 0.20, 0.20, TableState.check, minutes: 71, server: 'Jo', total: r'$142.15'),
    _Table('6', 0.04, 0.64, 0.20, 0.20, TableState.overdue, minutes: 38, server: 'Sam', total: r'$58.00'),
    _Table('7', 0.28, 0.64, 0.20, 0.20, TableState.ordered, seats: 6, minutes: 12, server: 'Maya', total: r'$64.25'),
    _Table('Bar', 0.58, 0.08, 0.36, 0.14, TableState.seated, seats: 8, minutes: 3, server: 'Ari'),
    _Table('Patio 1', 0.58, 0.40, 0.16, 0.20, TableState.free, round: true),
    _Table('Patio 2', 0.78, 0.40, 0.16, 0.20, TableState.free, round: true),
    _Table(
      'Booth',
      0.58,
      0.68,
      0.36,
      0.16,
      TableState.ordered,
      seats: 6,
      minutes: 18,
      server: 'Maya',
      total: r'$97.10',
    ),
  ];

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return RegisterScaffold(
      areas: const ['Dining room', 'Gift shop'],
      area: 0,
      staff: 'Sam',
      initials: 'SK',
      body: Padding(
        padding: const EdgeInsets.all(PosSpacing.lg),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                PosSegmented(options: const ['Main room', 'Patio', 'Bar'], selected: 0, onSelected: (_) {}),
                const Spacer(),
                for (final (label, tone) in const [
                  ('Free', PosTone.neutral),
                  ('Seated', PosTone.primary),
                  ('Ordered', PosTone.success),
                  ('Check', PosTone.warning),
                  ('Needs attention', PosTone.danger),
                ]) ...[PosStatusPill(label: label, tone: tone), const SizedBox(width: PosSpacing.xs + 2)],
              ],
            ),
            const SizedBox(height: PosSpacing.md),
            Expanded(
              child: PosSurface(
                color: c.surfaceMuted,
                child: LayoutBuilder(
                  builder: (context, box) {
                    return Stack(
                      children: [
                        for (final t in _tables)
                          Positioned(
                            left: t.x * box.maxWidth,
                            top: t.y * box.maxHeight,
                            width: t.w * box.maxWidth,
                            height: t.h * box.maxHeight,
                            child: _TableTile(table: t, selected: t.name == '5'),
                          ),
                      ],
                    );
                  },
                ),
              ),
            ),
          ],
        ),
      ),
      panel: OrderPanel(
        title: 'Table 5',
        subtitle: '4 guests · Jo · seated 71 min',
        header: PosSegmented(
          options: const ['All seats', '1', '2', '3', '4'],
          selected: 0,
          expand: true,
          onSelected: (_) {},
        ),
        lines: const [
          SampleLine('1', 'Burrata', r'$16.00', details: ['Course 1 · to share'], status: 'Served'),
          SampleLine('1', 'Ribeye', r'$42.00', details: ['Seat 1 · medium rare'], status: 'Served'),
          SampleLine('1', 'Halibut', r'$36.00', details: ['Seat 2'], status: 'Served'),
          SampleLine('2', 'Glass of Rioja', r'$28.00', details: ['Seats 1, 3'], status: 'Served'),
          SampleLine('1', 'Tiramisu', r'$11.00', details: ['Course 3 · seat 4'], status: 'Fire'),
          SampleLine('1', 'Mug (gift shop)', r'$12.00', details: ['Credited to gift shop'], status: 'Shop'),
        ],
        totals: const [
          ('Subtotal', r'$145.00'),
          ('Food & drink tax 5%', r'$6.65'),
          ('Sales tax 8.25%', r'$0.99'),
          ('Total', r'$152.64'),
        ],
        actions: const ['Split', 'Move', 'Print'],
        secondary: 'Send',
        primary: r'Pay $152.64',
      ),
    );
  }
}

class _TableTile extends StatelessWidget {
  const _TableTile({required this.table, required this.selected});

  final _Table table;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    final tone = switch (table.state) {
      TableState.free => PosTone.neutral,
      TableState.seated => PosTone.primary,
      TableState.ordered => PosTone.success,
      TableState.check => PosTone.warning,
      TableState.overdue => PosTone.danger,
    };
    final (fg, bg) = tone.colors(c);
    final free = table.state == TableState.free;
    return Padding(
      padding: const EdgeInsets.all(4),
      child: Material(
        color: free ? c.surface : Color.alphaBlend(bg, c.surface),
        shape: table.round && table.w < 0.2
            ? StadiumBorder(
                side: BorderSide(color: selected ? c.text : fg.withValues(alpha: 0.6), width: selected ? 3 : 1.5),
              )
            : RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(PosRadius.lg),
                side: BorderSide(color: selected ? c.text : fg.withValues(alpha: 0.6), width: selected ? 3 : 1.5),
              ),
        child: InkWell(
          onTap: () {},
          customBorder: const StadiumBorder(),
          child: Padding(
            padding: const EdgeInsets.all(PosSpacing.sm),
            child: FittedBox(
              fit: BoxFit.scaleDown,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  PosText(table.name, style: PosType.title, weight: FontWeight.w700),
                  PosText(
                    free ? '${table.seats} seats' : '${table.server} · ${table.minutes} min',
                    style: PosType.caption,
                    color: free ? c.textMuted : fg,
                    weight: FontWeight.w600,
                  ),
                  if (table.total != null) PosText(table.total!, style: PosType.caption, color: c.textMuted),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
