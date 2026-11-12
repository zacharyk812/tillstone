// Shared layout pieces for register-style screens (M1 designs).

import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

/// Category accent colors for item tiles. Chosen to stay distinguishable in
/// both themes and for common color-vision deficiencies.
abstract final class Accents {
  static const coffee = Color(0xFF8B5E3C);
  static const cold = Color(0xFF3D7EA6);
  static const bakery = Color(0xFFC08A2E);
  static const food = Color(0xFFB4533A);
  static const drinks = Color(0xFF7A4E9E);
  static const merch = Color(0xFF2E8A6E);
  static const books = Color(0xFF4A5D8C);
}

/// Top bar plus a body split into a working area and the order panel.
class RegisterScaffold extends StatelessWidget {
  const RegisterScaffold({
    super.key,
    required this.areas,
    required this.area,
    required this.body,
    this.panel,
    this.panelWidth = 400,
    this.staff = 'Maya',
    this.initials = 'MR',
    this.sync = PosSyncState.synced,
    this.syncLabel = '3 devices · all synced',
    this.onArea,
    this.onLock,
  });

  final List<String> areas;
  final int area;
  final Widget body;
  final Widget? panel;
  final double panelWidth;
  final String staff;
  final String initials;
  final PosSyncState sync;
  final String syncLabel;
  final ValueChanged<int>? onArea;
  final VoidCallback? onLock;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return Scaffold(
      body: SafeArea(
        child: Column(
          children: [
            PosTopBar(
              leading: areas.length > 1
                  ? PosSegmented(options: areas, selected: area, onSelected: onArea ?? (_) {})
                  : null,
              syncState: sync,
              syncLabel: syncLabel,
              staffName: staff,
              staffInitials: initials,
              onLock: onLock,
            ),
            Expanded(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(child: body),
                  if (panel != null)
                    Container(
                      width: panelWidth,
                      decoration: BoxDecoration(
                        color: c.surface,
                        border: Border(left: BorderSide(color: c.border)),
                      ),
                      child: panel,
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// A line as shown in the order panel.
class SampleLine {
  const SampleLine(
    this.qty,
    this.name,
    this.amount, {
    this.details = const [],
    this.tone,
    this.selected = false,
    this.status,
  });

  final String qty;
  final String name;
  final String amount;
  final List<String> details;
  final PosTone? tone;
  final bool selected;

  /// Small status chip, e.g. "Sent" or "Seat 2".
  final String? status;
}

/// The right-hand order panel: header, lines, totals and actions.
class OrderPanel extends StatelessWidget {
  const OrderPanel({
    super.key,
    required this.title,
    this.subtitle,
    this.header,
    required this.lines,
    required this.totals,
    required this.actions,
    required this.primary,
    this.secondary,
    this.onAction,
    this.onPrimary,
    this.onSecondary,
    this.onLine,
    this.empty,
  });

  final String title;
  final String? subtitle;
  final Widget? header;
  final List<SampleLine> lines;
  final List<(String, String)> totals;
  final List<String> actions;
  final String primary;
  final String? secondary;
  final ValueChanged<String>? onAction;
  final VoidCallback? onPrimary;
  final VoidCallback? onSecondary;
  final ValueChanged<int>? onLine;

  /// Shown instead of the lines when there are none.
  final Widget? empty;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(PosSpacing.lg, PosSpacing.lg, PosSpacing.lg, PosSpacing.sm + 2),
          child: PosHeading(title, subtitle: subtitle),
        ),
        if (header != null)
          Container(
            padding: const EdgeInsets.fromLTRB(PosSpacing.lg, 0, PosSpacing.lg, PosSpacing.md),
            decoration: BoxDecoration(
              border: Border(bottom: BorderSide(color: c.border)),
            ),
            child: header,
          ),
        Expanded(
          child: lines.isEmpty && empty != null
              ? Center(child: empty)
              : ListView(
                  children: [
                    for (final (i, l) in lines.indexed)
                      InkWell(
                        onTap: onLine == null ? null : () => onLine!(i),
                        child: PosOrderLine(
                          quantity: l.qty,
                          name: l.name,
                          amount: l.amount,
                          details: l.details,
                          tone: l.tone,
                          selected: l.selected,
                          trailing: l.status == null ? null : PosBadge(label: l.status!, tone: PosTone.neutral),
                        ),
                      ),
                  ],
                ),
        ),
        Container(
          padding: const EdgeInsets.fromLTRB(PosSpacing.lg, PosSpacing.md, PosSpacing.lg, PosSpacing.lg),
          decoration: BoxDecoration(
            border: Border(top: BorderSide(color: c.border)),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              for (final (i, (label, amount)) in totals.indexed)
                PosTotalRow(label: label, amount: amount, emphasis: i == totals.length - 1),
              const SizedBox(height: PosSpacing.sm + 2),
              Row(
                children: [
                  for (final (i, a) in actions.indexed) ...[
                    if (i > 0) const SizedBox(width: PosSpacing.sm),
                    Expanded(
                      child: PosButton(
                        label: a,
                        kind: PosButtonKind.secondary,
                        onPressed: onAction == null ? () {} : () => onAction!(a),
                      ),
                    ),
                  ],
                ],
              ),
              const SizedBox(height: PosSpacing.sm),
              Row(
                children: [
                  if (secondary != null) ...[
                    Expanded(
                      child: _OutlineButton(label: secondary!, onTap: onSecondary),
                    ),
                    const SizedBox(width: PosSpacing.sm),
                  ],
                  Expanded(
                    flex: 2,
                    child: PosButton(label: primary, large: true, onPressed: onPrimary ?? () {}),
                  ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _OutlineButton extends StatelessWidget {
  const _OutlineButton({required this.label, this.onTap});

  final String label;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return Material(
      color: Colors.transparent,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(PosRadius.md),
        side: BorderSide(color: c.primary, width: 2),
      ),
      child: InkWell(
        onTap: onTap ?? () {},
        borderRadius: BorderRadius.circular(PosRadius.md),
        child: SizedBox(
          height: PosSize.primaryTapTarget,
          child: Center(
            child: PosText(label, style: PosType.label, color: c.primary, weight: FontWeight.w700, size: 18),
          ),
        ),
      ),
    );
  }
}

/// An item grid that fills the available width with tiles of a minimum size.
class ItemGrid extends StatelessWidget {
  const ItemGrid({super.key, required this.children, this.minTileWidth = 160, this.tileHeight = 108});

  final List<Widget> children;
  final double minTileWidth;
  final double tileHeight;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, box) {
        final columns = (box.maxWidth / (minTileWidth + PosSpacing.md)).floor().clamp(2, 6);
        return GridView.count(
          crossAxisCount: columns,
          mainAxisSpacing: PosSpacing.md,
          crossAxisSpacing: PosSpacing.md,
          childAspectRatio: ((box.maxWidth - PosSpacing.md * (columns - 1)) / columns) / tileHeight,
          children: children,
        );
      },
    );
  }
}
