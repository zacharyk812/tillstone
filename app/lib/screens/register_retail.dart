import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

import 'common.dart';

/// Retail register: scan or search first, stock shown on tiles, customer
/// lookup and returns close at hand.
class RetailRegister extends StatelessWidget {
  const RetailRegister({super.key});

  static const _results = [
    ('The Overstory', r'$18.99', Accents.books, '12 in stock', 0, false),
    ('Braiding Sweetgrass', r'$20.00', Accents.books, '4 in stock', 1, false),
    ('Piranesi', r'$17.00', Accents.books, '9 in stock', 0, false),
    ('Klara and the Sun', r'$16.95', Accents.books, 'Last one', 0, false),
    ('A Psalm for the Wild-Built', r'$15.99', Accents.books, '0 in stock', 0, true),
    ('Tote bag', r'$14.00', Accents.merch, '31 in stock', 1, false),
    ('Enamel mug', r'$12.00', Accents.merch, '18 in stock', 0, false),
    ('Gift card', r'Any amount', Accents.merch, null, 0, false),
  ];

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return RegisterScaffold(
      areas: const ['Café', 'Books'],
      area: 1,
      body: Padding(
        padding: const EdgeInsets.all(PosSpacing.lg),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const PosSearchBar(hint: 'Scan a barcode, or search title, author or SKU'),
            const SizedBox(height: PosSpacing.md),
            Row(
              children: [
                const PosText('Recently scanned and popular', style: PosType.label),
                const Spacer(),
                PosButton(label: 'Return or exchange', kind: PosButtonKind.secondary, onPressed: () {}),
                const SizedBox(width: PosSpacing.sm),
                PosButton(label: 'Quick sale', kind: PosButtonKind.secondary, onPressed: () {}),
              ],
            ),
            const SizedBox(height: PosSpacing.md),
            Expanded(
              child: ItemGrid(
                minTileWidth: 190,
                children: [
                  for (final (name, price, accent, stock, count, out) in _results)
                    PosItemTile(
                      name: name,
                      price: price,
                      accent: accent,
                      detail: stock,
                      count: count,
                      soldOut: out,
                      onTap: () {},
                    ),
                ],
              ),
            ),
            PosSurface(
              padding: const EdgeInsets.all(PosSpacing.md),
              child: Row(
                children: [
                  Icon(Icons.qr_code_scanner, color: c.success),
                  const SizedBox(width: PosSpacing.sm),
                  PosText('Scanner connected · last scan: 9781250272188', style: PosType.caption, color: c.textMuted),
                ],
              ),
            ),
          ],
        ),
      ),
      panel: OrderPanel(
        title: 'Sale B-0217',
        subtitle: 'Books',
        header: PosSurface(
          padding: const EdgeInsets.symmetric(horizontal: PosSpacing.md, vertical: PosSpacing.sm + 2),
          child: Row(
            children: [
              Icon(Icons.person_add_alt, size: 20, color: c.textMuted),
              const SizedBox(width: PosSpacing.sm),
              PosText('Add customer for receipt or rewards', color: c.textMuted),
            ],
          ),
        ),
        lines: const [
          SampleLine('1', 'Braiding Sweetgrass', r'$20.00', details: ['Paperback · 9781571313560']),
          SampleLine('1', 'Tote bag', r'$14.00', details: ['Natural canvas']),
          SampleLine('1', 'Latte', r'$4.75', details: ['From the café · food tax'], status: 'Café'),
        ],
        totals: const [
          ('Subtotal', r'$38.75'),
          ('Sales tax 8.25%', r'$2.81'),
          ('Food tax 5%', r'$0.24'),
          ('Total', r'$41.80'),
        ],
        actions: const ['Discount', 'Receipt', 'Hold'],
        primary: r'Pay $41.80',
      ),
    );
  }
}
