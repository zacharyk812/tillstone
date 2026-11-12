import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

import '../screens/common.dart';
import '../src/rust/api/register.dart';
import '../src/session.dart';
import 'payment.dart';

/// The register, wired to the Rust core: real catalog, real orders, real totals.
class RegisterScreen extends StatelessWidget {
  const RegisterScreen({super.key, required this.session});

  final Session session;

  static const _accents = [Accents.coffee, Accents.bakery, Accents.cold, Accents.books, Accents.merch, Accents.food];

  Future<void> _tapItem(BuildContext context, ItemView item) async {
    if (item.variants.isEmpty && item.optionGroups.isEmpty) {
      await session.addItem(item);
    } else {
      final choice = await showDialog<(String?, List<String>)>(
        context: context,
        builder: (_) => _OptionsDialog(item: item),
      );
      if (choice != null) await session.addItem(item, variantId: choice.$1, optionIds: choice.$2);
    }
    if (context.mounted) _showError(context);
  }

  void _showError(BuildContext context) {
    final e = session.error;
    if (e != null && context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(e)));
    }
  }

  Future<void> _editLine(BuildContext context, LineView line) async {
    final qty = await showDialog<int>(
      context: context,
      builder: (_) => _QuantityDialog(line: line),
    );
    if (qty != null) {
      await session.setQuantity(line, qty);
      if (context.mounted) _showError(context);
    }
  }

  Future<void> _action(BuildContext context, String action) async {
    switch (action) {
      case 'Staff 10%':
        await session.discount('Staff 10%', 1000);
      case 'Clear':
        session.newOrder();
    }
    if (context.mounted) _showError(context);
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: session,
      builder: (context, _) {
        final centers = session.centers;
        final area = centers.indexWhere((c) => c.id == session.center?.id);
        final order = session.order;
        final items = session.items;
        int countOf(ItemView item) =>
            order?.lines
                .where((l) => l.name == item.name || l.name.startsWith('${item.name} ('))
                .fold<int>(0, (n, l) => n + l.quantity.toInt()) ??
            0;

        return PopScope(
          canPop: false,
          child: RegisterScaffold(
            areas: [for (final c in centers) c.name],
            area: area < 0 ? 0 : area,
            onArea: (i) => session.selectCenter(centers[i]),
            staff: session.staff?.name ?? '',
            initials: session.staff?.initials ?? '',
            sync: PosSyncState.offline,
            syncLabel: session.till.training() ? 'Training mode · this device only' : 'This device only',
            onLock: () {
              session.signOut();
              Navigator.of(context).pop();
            },
            body: Padding(
              padding: const EdgeInsets.all(PosSpacing.lg),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const PosSearchBar(hint: 'Search items or scan a barcode'),
                  const SizedBox(height: PosSpacing.md),
                  Expanded(
                    child: ItemGrid(
                      minTileWidth: 170,
                      children: [
                        for (final (i, item) in items.indexed)
                          PosItemTile(
                            key: ValueKey('item-${item.name}'),
                            name: item.name,
                            price: item.variants.isEmpty ? item.price : 'from ${item.variants.first.price}',
                            accent: _accents[(i ~/ 3) % _accents.length],
                            detail: item.stock == null ? null : '${item.stock} in stock',
                            soldOut: item.stock != null && item.stock! <= 0,
                            count: countOf(item),
                            onTap: session.busy ? null : () => _tapItem(context, item),
                          ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
            panel: OrderPanel(
              title: order == null ? 'New order' : 'Order ${order.number}',
              subtitle: session.center?.name,
              empty: PosText('Tap an item to start', color: PosThemeColors.of(context).textMuted),
              lines: [
                for (final l in order?.lines ?? const <LineView>[])
                  SampleLine('${l.quantity}', l.name, l.amount, details: l.details),
                if (order?.discount != null) SampleLine('%', 'Discount', order!.discount!, tone: PosTone.success),
              ],
              onLine: (i) {
                final lines = order?.lines ?? const <LineView>[];
                if (i < lines.length) _editLine(context, lines[i]);
              },
              totals: [
                ('Subtotal', order?.subtotal ?? r'$0.00'),
                if (order?.discount != null) ('Discounts', order!.discount!),
                for (final (name, amount) in order?.taxes ?? const <(String, String)>[]) (name, amount),
                ('Total', order?.total ?? r'$0.00'),
              ],
              actions: const ['Staff 10%', 'Clear'],
              onAction: (a) => _action(context, a),
              primary: order == null ? 'Pay' : 'Pay ${order.balance}',
              onPrimary: order == null || order.lines.isEmpty
                  ? null
                  : () =>
                        Navigator.of(context)
                            .push(MaterialPageRoute<void>(builder: (_) => PaymentScreenLive(session: session))),
            ),
          ),
        );
      },
    );
  }
}

class _OptionsDialog extends StatefulWidget {
  const _OptionsDialog({required this.item});

  final ItemView item;

  @override
  State<_OptionsDialog> createState() => _OptionsDialogState();
}

class _OptionsDialogState extends State<_OptionsDialog> {
  String? _variant;
  final _options = <String>{};

  @override
  void initState() {
    super.initState();
    if (widget.item.variants.isNotEmpty) _variant = widget.item.variants.first.id;
  }

  @override
  Widget build(BuildContext context) {
    final item = widget.item;
    return AlertDialog(
      title: PosText(item.name, style: PosType.title),
      content: SizedBox(
        width: 420,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              if (item.variants.isNotEmpty) ...[
                const PosText('Size', style: PosType.label),
                const SizedBox(height: PosSpacing.sm),
                PosSegmented(
                  options: [for (final v in item.variants) '${v.name} ${v.price}'],
                  selected: item.variants.indexWhere((v) => v.id == _variant),
                  expand: true,
                  onSelected: (i) => setState(() => _variant = item.variants[i].id),
                ),
                const SizedBox(height: PosSpacing.lg),
              ],
              for (final g in item.optionGroups) ...[
                PosText(g.name, style: PosType.label),
                Wrap(
                  spacing: PosSpacing.sm,
                  children: [
                    for (final o in g.options)
                      FilterChip(
                        label: Text(o.price == r'$0.00' ? o.name : '${o.name} +${o.price}'),
                        selected: _options.contains(o.id),
                        onSelected: (on) => setState(() {
                          if (on) {
                            if (g.max == 1) _options.removeAll(g.options.map((x) => x.id));
                            _options.add(o.id);
                          } else {
                            _options.remove(o.id);
                          }
                        }),
                      ),
                  ],
                ),
                const SizedBox(height: PosSpacing.md),
              ],
            ],
          ),
        ),
      ),
      actions: [
        PosButton(label: 'Cancel', kind: PosButtonKind.secondary, onPressed: () => Navigator.of(context).pop()),
        PosButton(label: 'Add', onPressed: () => Navigator.of(context).pop((_variant, _options.toList()))),
      ],
    );
  }
}

class _QuantityDialog extends StatefulWidget {
  const _QuantityDialog({required this.line});

  final LineView line;

  @override
  State<_QuantityDialog> createState() => _QuantityDialogState();
}

class _QuantityDialogState extends State<_QuantityDialog> {
  late int _qty = widget.line.quantity.toInt();

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: PosText(widget.line.name, style: PosType.title),
      content: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          PosButton(
            label: '−',
            kind: PosButtonKind.secondary,
            onPressed: _qty > 0 ? () => setState(() => _qty--) : null,
          ),
          SizedBox(
            width: 72,
            child: PosText('$_qty', style: PosType.display, align: TextAlign.center),
          ),
          PosButton(label: '+', kind: PosButtonKind.secondary, onPressed: () => setState(() => _qty++)),
        ],
      ),
      actions: [
        PosButton(label: 'Remove', kind: PosButtonKind.danger, onPressed: () => Navigator.of(context).pop(0)),
        PosButton(label: 'Done', onPressed: () => Navigator.of(context).pop(_qty)),
      ],
    );
  }
}
