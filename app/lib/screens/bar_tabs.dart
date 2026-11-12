import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

import 'common.dart';

/// Bar: open tabs by name on the left, a quick drink grid, the selected tab
/// on the right. Cards are pre-authorized when a tab opens.
class BarTabs extends StatelessWidget {
  const BarTabs({super.key});

  static const _tabs = [
    ('Priya', r'$38.50', '1h 12m', true, false),
    ('Marcus + 3', r'$129.90', '48m', true, true),
    ('Table by window', r'$24.00', '35m', false, false),
    ('Lee', r'$9.00', '6m', true, false),
    ('Dana', r'$61.75', '2h 40m', true, true),
  ];

  static const _drinks = [
    ('House lager', r'$7.00', Accents.drinks),
    ('IPA', r'$8.00', Accents.drinks),
    ('Stout', r'$8.00', Accents.drinks),
    ('Cider', r'$7.50', Accents.drinks),
    ('Old fashioned', r'$13.00', Accents.food),
    ('Negroni', r'$13.00', Accents.food),
    ('Margarita', r'$12.00', Accents.food),
    ('House red', r'$10.00', Accents.coffee),
    ('Soda', r'$3.00', Accents.cold),
    ('Fries', r'$6.00', Accents.bakery),
  ];

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return RegisterScaffold(
      areas: const ['Taproom', 'Merch'],
      area: 0,
      staff: 'Ari',
      initials: 'AL',
      body: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Container(
            width: 280,
            decoration: BoxDecoration(
              border: Border(right: BorderSide(color: c.border)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Padding(
                  padding: const EdgeInsets.all(PosSpacing.lg),
                  child: PosButton(label: '+  New tab', large: true, onPressed: () {}),
                ),
                Expanded(
                  child: ListView(
                    children: [
                      for (final (i, (name, total, age, card, stale)) in _tabs.indexed)
                        Container(
                          padding: const EdgeInsets.symmetric(horizontal: PosSpacing.lg, vertical: PosSpacing.md),
                          decoration: BoxDecoration(
                            color: i == 1 ? c.primary.withValues(alpha: 0.07) : null,
                            border: Border(
                              left: BorderSide(color: i == 1 ? c.primary : Colors.transparent, width: 4),
                              bottom: BorderSide(color: c.surfaceMuted),
                            ),
                          ),
                          child: Row(
                            children: [
                              Expanded(
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    PosText(name, style: PosType.label),
                                    const SizedBox(height: 2),
                                    Wrap(
                                      spacing: PosSpacing.sm,
                                      runSpacing: 4,
                                      crossAxisAlignment: WrapCrossAlignment.center,
                                      children: [
                                        PosText(age, style: PosType.caption, color: c.textMuted),
                                        if (card) Icon(Icons.credit_card, size: 14, color: c.textMuted),
                                        if (stale)
                                          const PosStatusPill(label: 'Hold expires soon', tone: PosTone.warning),
                                      ],
                                    ),
                                  ],
                                ),
                              ),
                              PosText(total, style: PosType.label),
                            ],
                          ),
                        ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.all(PosSpacing.lg),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  PosCategoryTabs(
                    categories: const ['Draft', 'Cocktails', 'Wine', 'Soft', 'Snacks'],
                    selected: 0,
                    onSelected: (_) {},
                  ),
                  const SizedBox(height: PosSpacing.md),
                  Expanded(
                    child: ItemGrid(
                      minTileWidth: 140,
                      tileHeight: 96,
                      children: [
                        for (final (name, price, accent) in _drinks)
                          PosItemTile(name: name, price: price, accent: accent, onTap: () {}),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
      panel: OrderPanel(
        title: 'Marcus + 3',
        subtitle: 'Card on file ····4242 · held \$150',
        lines: const [
          SampleLine('4', 'IPA', r'$32.00', status: 'Sent'),
          SampleLine('2', 'Old fashioned', r'$26.00', status: 'Sent'),
          SampleLine('2', 'Negroni', r'$26.00', status: 'Sent'),
          SampleLine('2', 'Fries', r'$12.00', details: ['1 with aioli'], status: 'Sent'),
          SampleLine('2', 'Margarita', r'$24.00', details: ['Salt rim'], selected: true),
        ],
        totals: const [('Subtotal', r'$120.00'), ('Tax 8.25%', r'$9.90'), ('Total', r'$129.90')],
        actions: const ['Transfer', 'Split', 'Print'],
        secondary: 'Send',
        primary: 'Close tab',
      ),
    );
  }
}
