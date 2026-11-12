import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

import 'common.dart';

/// Quick-service register: big item grid, modifiers, dining option, Send + Pay.
class QuickServiceRegister extends StatefulWidget {
  const QuickServiceRegister({super.key});

  @override
  State<QuickServiceRegister> createState() => _QuickServiceRegisterState();
}

class _QuickServiceRegisterState extends State<QuickServiceRegister> {
  int _category = 0;
  int _dining = 0;

  static const _items = [
    ('Espresso', r'$3.00', Accents.coffee, 0, false),
    ('Americano', r'$3.50', Accents.coffee, 0, false),
    ('Latte', r'$4.75', Accents.coffee, 1, false),
    ('Cappuccino', r'$4.50', Accents.coffee, 0, false),
    ('Flat white', r'$4.50', Accents.coffee, 0, false),
    ('Mocha', r'$5.25', Accents.coffee, 0, false),
    ('Cortado', r'$4.00', Accents.coffee, 0, false),
    ('Pumpkin spice latte', r'$5.75', Accents.coffee, 0, true),
    ('Cold brew', r'$4.50', Accents.cold, 1, false),
    ('Iced latte', r'$5.00', Accents.cold, 0, false),
    ('Butter croissant', r'$3.50', Accents.bakery, 2, false),
    ('Blueberry muffin', r'$3.75', Accents.bakery, 0, false),
  ];

  @override
  Widget build(BuildContext context) {
    return RegisterScaffold(
      areas: const ['Café', 'Books'],
      area: 0,
      body: Padding(
        padding: const EdgeInsets.all(PosSpacing.lg),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const PosSearchBar(hint: 'Search items or scan a barcode'),
            const SizedBox(height: PosSpacing.md),
            PosCategoryTabs(
              categories: const ['Coffee', 'Tea', 'Bakery', 'Breakfast', 'Cold drinks', 'Beans'],
              selected: _category,
              onSelected: (i) => setState(() => _category = i),
            ),
            const SizedBox(height: PosSpacing.md),
            Expanded(
              child: ItemGrid(
                children: [
                  for (final (name, price, accent, count, soldOut) in _items)
                    PosItemTile(name: name, price: price, accent: accent, count: count, soldOut: soldOut, onTap: () {}),
                ],
              ),
            ),
          ],
        ),
      ),
      panel: OrderPanel(
        title: 'Order A42',
        subtitle: 'Café · opened 9:41',
        header: PosSegmented(
          options: const ['For here', 'To go', 'Pickup'],
          selected: _dining,
          expand: true,
          onSelected: (i) => setState(() => _dining = i),
        ),
        lines: const [
          SampleLine(
            '1',
            'Latte',
            r'$6.25',
            details: [r'Large · Oat milk +$0.75', r'Extra shot +$0.75'],
            selected: true,
          ),
          SampleLine('2', 'Butter croissant', r'$7.00', details: ['Warmed']),
          SampleLine('1', 'Cold brew', r'$4.50'),
          SampleLine('%', 'Staff discount 10%', r'−$1.78', tone: PosTone.success),
        ],
        totals: const [
          ('Subtotal', r'$17.75'),
          ('Discounts', r'−$1.78'),
          ('Sales tax 8.25%', r'$1.32'),
          ('Total', r'$17.29'),
        ],
        actions: const ['Discount', 'Note', 'Hold'],
        secondary: 'Send',
        primary: r'Pay $17.29',
      ),
    );
  }
}
