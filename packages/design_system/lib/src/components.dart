import 'package:material_ui/material_ui.dart';

import 'theme.dart';
import 'tokens.g.dart';

TextStyle _text(BuildContext context, PosTypeStyle t, {Color? color}) => TextStyle(
  fontFamily: PosTheme.fontFamily,
  package: PosTheme.fontPackage,
  fontSize: t.size,
  fontWeight: t.weight,
  height: t.height,
  color: color ?? PosThemeColors.of(context).text,
);

/// A plain surface with the standard border and radius.
class PosSurface extends StatelessWidget {
  const PosSurface({super.key, required this.child, this.padding, this.color, this.radius = PosRadius.md});

  final Widget child;
  final EdgeInsetsGeometry? padding;
  final Color? color;
  final double radius;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return Container(
      padding: padding,
      decoration: BoxDecoration(
        color: color ?? c.surface,
        borderRadius: BorderRadius.circular(radius),
        border: Border.all(color: c.border),
      ),
      child: child,
    );
  }
}

enum PosSyncState { synced, pending, offline, error }

/// Shows how this device is syncing with its peers (PLAN §6 "Status").
class PosSyncIndicator extends StatelessWidget {
  const PosSyncIndicator({super.key, required this.state, required this.label});

  final PosSyncState state;
  final String label;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    final dot = switch (state) {
      PosSyncState.synced => c.success,
      PosSyncState.pending => c.warning,
      PosSyncState.offline => c.textMuted,
      PosSyncState.error => c.danger,
    };
    return Semantics(
      label: 'Sync: $label',
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 9,
            height: 9,
            decoration: BoxDecoration(color: dot, shape: BoxShape.circle),
          ),
          const SizedBox(width: PosSpacing.sm),
          Text(label, style: _text(context, PosType.caption, color: c.textMuted)),
        ],
      ),
    );
  }
}

/// A two-to-five option segmented control (selling areas, dining options).
class PosSegmented extends StatelessWidget {
  const PosSegmented({
    super.key,
    required this.options,
    required this.selected,
    required this.onSelected,
    this.expand = false,
  });

  final List<String> options;
  final int selected;
  final ValueChanged<int> onSelected;

  /// Fill the available width with equal segments.
  final bool expand;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    Widget segment(int i) {
      final on = i == selected;
      final child = Semantics(
        button: true,
        selected: on,
        child: InkWell(
          onTap: () => onSelected(i),
          borderRadius: BorderRadius.circular(PosRadius.sm),
          child: AnimatedContainer(
            duration: PosMotion.fast,
            constraints: const BoxConstraints(minHeight: PosSize.minTapTarget - 8),
            alignment: Alignment.center,
            padding: const EdgeInsets.symmetric(horizontal: PosSpacing.lg),
            decoration: BoxDecoration(
              color: on ? c.surface : Colors.transparent,
              borderRadius: BorderRadius.circular(PosRadius.sm),
              boxShadow: on ? const [BoxShadow(color: Color(0x1F000000), blurRadius: 2, offset: Offset(0, 1))] : null,
            ),
            child: Text(options[i], style: _text(context, PosType.label, color: on ? c.text : c.textMuted)),
          ),
        ),
      );
      return expand ? Expanded(child: child) : child;
    }

    return Container(
      padding: const EdgeInsets.all(3),
      decoration: BoxDecoration(color: c.surfaceMuted, borderRadius: BorderRadius.circular(PosRadius.md)),
      child: Row(
        mainAxisSize: expand ? MainAxisSize.max : MainAxisSize.min,
        children: [for (var i = 0; i < options.length; i++) segment(i)],
      ),
    );
  }
}

/// The bar across the top of the register.
class PosTopBar extends StatelessWidget {
  const PosTopBar({
    super.key,
    this.center,
    this.leading,
    required this.syncState,
    required this.syncLabel,
    required this.staffName,
    required this.staffInitials,
    this.onLock,
  });

  final Widget? leading;
  final Widget? center;
  final PosSyncState syncState;
  final String syncLabel;
  final String staffName;
  final String staffInitials;
  final VoidCallback? onLock;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return Container(
      height: 60,
      padding: const EdgeInsets.symmetric(horizontal: PosSpacing.lg),
      decoration: BoxDecoration(
        color: c.surface,
        border: Border(bottom: BorderSide(color: c.border)),
      ),
      child: Row(
        children: [
          const PosBrand(),
          if (leading != null) ...[const SizedBox(width: PosSpacing.lg), leading!],
          const Spacer(),
          if (center != null) ...[center!, const Spacer()],
          PosSyncIndicator(state: syncState, label: syncLabel),
          const SizedBox(width: PosSpacing.lg),
          Container(
            padding: const EdgeInsets.fromLTRB(6, 6, 12, 6),
            decoration: BoxDecoration(color: c.surfaceMuted, borderRadius: BorderRadius.circular(999)),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                CircleAvatar(
                  radius: 15,
                  backgroundColor: c.primary.withValues(alpha: 0.15),
                  child: Text(
                    staffInitials,
                    style: _text(context, PosType.caption, color: c.primary).copyWith(fontWeight: FontWeight.w700),
                  ),
                ),
                const SizedBox(width: PosSpacing.sm),
                Text(staffName, style: _text(context, PosType.label)),
              ],
            ),
          ),
          const SizedBox(width: PosSpacing.sm),
          PosIconButton(icon: Icons.lock_outline, label: 'Lock', onPressed: onLock ?? () {}),
        ],
      ),
    );
  }
}

/// The product's mark and name (from tokens.json, so forks can rebrand).
class PosBrand extends StatelessWidget {
  const PosBrand({super.key});

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: 28,
          height: 28,
          decoration: BoxDecoration(color: c.primary, borderRadius: BorderRadius.circular(8)),
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 8),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: c.primary.withValues(alpha: 0.0),
              borderRadius: BorderRadius.circular(3),
              border: Border.all(color: c.onPrimary.withValues(alpha: 0.75), width: 2),
            ),
          ),
        ),
        const SizedBox(width: PosSpacing.sm),
        Text(
          PosBrandInfo.name,
          style: _text(context, PosType.title).copyWith(fontSize: 18, fontWeight: FontWeight.w700),
        ),
      ],
    );
  }
}

/// A square-ish icon button with a 48px target.
class PosIconButton extends StatelessWidget {
  const PosIconButton({super.key, required this.icon, required this.label, required this.onPressed});

  final IconData icon;
  final String label;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return Semantics(
      button: true,
      label: label,
      child: Tooltip(
        message: label,
        child: InkWell(
          onTap: onPressed,
          borderRadius: BorderRadius.circular(PosRadius.md),
          child: Container(
            width: PosSize.minTapTarget,
            height: PosSize.minTapTarget,
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(PosRadius.md),
              border: Border.all(color: c.border),
            ),
            child: Icon(icon, size: 20, color: c.textMuted),
          ),
        ),
      ),
    );
  }
}

/// Search or scan. A scanner in keyboard mode types into this.
class PosSearchBar extends StatelessWidget {
  const PosSearchBar({super.key, required this.hint, this.shortcut = '/'});

  final String hint;
  final String shortcut;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return PosSurface(
      padding: const EdgeInsets.symmetric(horizontal: PosSpacing.md + 2),
      child: SizedBox(
        height: PosSize.minTapTarget,
        child: Row(
          children: [
            Icon(Icons.search, size: 20, color: c.textMuted),
            const SizedBox(width: PosSpacing.sm),
            Expanded(
              child: Text(hint, style: _text(context, PosType.body, color: c.textMuted)),
            ),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
              decoration: BoxDecoration(
                border: Border.all(color: c.border),
                borderRadius: BorderRadius.circular(6),
              ),
              child: Text(shortcut, style: _text(context, PosType.caption, color: c.textMuted)),
            ),
          ],
        ),
      ),
    );
  }
}

/// Category tabs above the item grid.
class PosCategoryTabs extends StatelessWidget {
  const PosCategoryTabs({super.key, required this.categories, required this.selected, required this.onSelected});

  final List<String> categories;
  final int selected;
  final ValueChanged<int> onSelected;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: Row(
        children: [
          for (var i = 0; i < categories.length; i++)
            Padding(
              padding: const EdgeInsets.only(right: PosSpacing.sm),
              child: Material(
                color: i == selected ? c.primary : c.surface,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(PosRadius.md),
                  side: BorderSide(color: i == selected ? c.primary : c.border),
                ),
                child: InkWell(
                  onTap: () => onSelected(i),
                  borderRadius: BorderRadius.circular(PosRadius.md),
                  child: Container(
                    constraints: const BoxConstraints(minHeight: PosSize.minTapTarget),
                    padding: const EdgeInsets.symmetric(horizontal: PosSpacing.lg + 2),
                    alignment: Alignment.center,
                    child: Text(
                      categories[i],
                      style: _text(context, PosType.label, color: i == selected ? c.onPrimary : c.text),
                    ),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

/// One item on the register grid.
class PosItemTile extends StatelessWidget {
  const PosItemTile({
    super.key,
    required this.name,
    required this.price,
    required this.accent,
    this.count = 0,
    this.soldOut = false,
    this.detail,
    this.onTap,
  });

  final String name;
  final String price;
  final Color accent;

  /// How many are already on the current order.
  final int count;
  final bool soldOut;

  /// Extra line such as "12 in stock" or a SKU.
  final String? detail;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return Opacity(
      opacity: soldOut ? 0.5 : 1,
      child: Material(
        color: c.surface,
        clipBehavior: Clip.antiAlias,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(PosRadius.md),
          side: BorderSide(color: c.border),
        ),
        child: InkWell(
          onTap: soldOut ? null : onTap,
          child: Stack(
            children: [
              Positioned(left: 0, top: 0, bottom: 0, child: Container(width: 6, color: accent)),
              Padding(
                padding: const EdgeInsets.fromLTRB(
                  PosSpacing.lg + 2,
                  PosSpacing.md + 2,
                  PosSpacing.md + 2,
                  PosSpacing.md + 2,
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Padding(
                      // Leave room for the count badge.
                      padding: EdgeInsets.only(right: count > 0 ? 26 : 0),
                      child: Text(
                        name,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: _text(context, PosType.label),
                      ),
                    ),
                    Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        if (detail != null) Text(detail!, style: _text(context, PosType.caption, color: c.textMuted)),
                        Text.rich(
                          TextSpan(
                            children: [
                              TextSpan(text: price),
                              if (soldOut)
                                TextSpan(
                                  text: ' · Sold out',
                                  style: TextStyle(color: c.danger, fontWeight: FontWeight.w600),
                                ),
                            ],
                          ),
                          style: _text(context, PosType.body, color: c.textMuted),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
              if (count > 0) Positioned(right: 10, top: 10, child: PosBadge(label: '$count')),
            ],
          ),
        ),
      ),
    );
  }
}

/// A small count or label badge.
class PosBadge extends StatelessWidget {
  const PosBadge({super.key, required this.label, this.tone = PosTone.primary});

  final String label;
  final PosTone tone;

  @override
  Widget build(BuildContext context) {
    final (fg, bg) = tone.colors(PosThemeColors.of(context));
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 3),
      decoration: BoxDecoration(color: bg, borderRadius: BorderRadius.circular(6)),
      child: Text(
        label,
        style: _text(context, PosType.caption, color: fg).copyWith(fontWeight: FontWeight.w700),
      ),
    );
  }
}

enum PosTone {
  primary,
  success,
  warning,
  danger,
  neutral;

  (Color, Color) colors(PosColors c) => switch (this) {
    PosTone.primary => (c.primary, c.primary.withValues(alpha: 0.12)),
    PosTone.success => (c.success, c.success.withValues(alpha: 0.12)),
    PosTone.warning => (c.warning, c.warning.withValues(alpha: 0.14)),
    PosTone.danger => (c.danger, c.danger.withValues(alpha: 0.12)),
    PosTone.neutral => (c.textMuted, c.surfaceMuted),
  };
}

/// A status pill, e.g. "Seated 42 min" or "Overdue".
class PosStatusPill extends StatelessWidget {
  const PosStatusPill({super.key, required this.label, required this.tone});

  final String label;
  final PosTone tone;

  @override
  Widget build(BuildContext context) {
    final (fg, bg) = tone.colors(PosThemeColors.of(context));
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(color: bg, borderRadius: BorderRadius.circular(999)),
      child: Text(
        label,
        style: _text(context, PosType.caption, color: fg).copyWith(fontWeight: FontWeight.w600),
      ),
    );
  }
}

/// One line of an order in the order panel.
class PosOrderLine extends StatelessWidget {
  const PosOrderLine({
    super.key,
    required this.quantity,
    required this.name,
    required this.amount,
    this.details = const [],
    this.selected = false,
    this.tone,
    this.trailing,
  });

  final String quantity;
  final String name;
  final String amount;
  final List<String> details;
  final bool selected;

  /// Colors the name and amount, e.g. green for discounts, red for voids.
  final PosTone? tone;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    final color = tone?.colors(c).$1;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: PosSpacing.lg, vertical: PosSpacing.md),
      decoration: BoxDecoration(
        color: selected ? c.primary.withValues(alpha: 0.07) : null,
        border: Border(
          left: BorderSide(color: selected ? c.primary : Colors.transparent, width: 4),
          bottom: BorderSide(color: c.surfaceMuted),
        ),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 32,
            height: 32,
            alignment: Alignment.center,
            decoration: BoxDecoration(color: c.surfaceMuted, borderRadius: BorderRadius.circular(8)),
            child: Text(quantity, style: _text(context, PosType.label)),
          ),
          const SizedBox(width: PosSpacing.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(name, style: _text(context, PosType.label, color: color)),
                for (final d in details)
                  Padding(
                    padding: const EdgeInsets.only(top: 2),
                    child: Text(d, style: _text(context, PosType.caption, color: c.textMuted)),
                  ),
              ],
            ),
          ),
          if (trailing != null) ...[trailing!, const SizedBox(width: PosSpacing.sm)],
          Text(
            amount,
            style: _text(
              context,
              PosType.label,
              color: color,
            ).copyWith(fontFeatures: const [FontFeature.tabularFigures()]),
          ),
        ],
      ),
    );
  }
}

/// A label/amount row in a totals block.
class PosTotalRow extends StatelessWidget {
  const PosTotalRow({super.key, required this.label, required this.amount, this.emphasis = false});

  final String label;
  final String amount;
  final bool emphasis;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    final style = emphasis
        ? _text(context, PosType.title).copyWith(fontSize: 20, fontWeight: FontWeight.w700)
        : _text(context, PosType.body, color: c.textMuted).copyWith(fontSize: 15);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        children: [
          Expanded(child: Text(label, style: style)),
          Text(amount, style: style.copyWith(fontFeatures: const [FontFeature.tabularFigures()])),
        ],
      ),
    );
  }
}

/// The on-screen number pad. The OS keyboard never appears on register
/// screens (PLAN §11).
class PosNumpad extends StatelessWidget {
  const PosNumpad({super.key, required this.onKey, this.bottomLeft = '00', this.keyHeight = 64});

  /// Called with '0'..'9', the bottom-left label, or 'back'.
  final ValueChanged<String> onKey;
  final String bottomLeft;
  final double keyHeight;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    Widget key(String k) => Expanded(
      child: Padding(
        padding: const EdgeInsets.all(4),
        child: Material(
          color: c.surfaceMuted,
          borderRadius: BorderRadius.circular(PosRadius.md),
          child: InkWell(
            onTap: () => onKey(k),
            borderRadius: BorderRadius.circular(PosRadius.md),
            child: SizedBox(
              height: keyHeight,
              child: Center(
                child: k == 'back'
                    ? Icon(Icons.backspace_outlined, color: c.text, semanticLabel: 'Delete')
                    : Text(k, style: _text(context, PosType.title).copyWith(fontSize: 24, fontWeight: FontWeight.w600)),
              ),
            ),
          ),
        ),
      ),
    );
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        for (final row in [
          ['1', '2', '3'],
          ['4', '5', '6'],
          ['7', '8', '9'],
          [bottomLeft, '0', 'back'],
        ])
          Row(children: [for (final k in row) key(k)]),
      ],
    );
  }
}

/// Filled/empty dots showing how many PIN digits were entered.
class PosPinDots extends StatelessWidget {
  const PosPinDots({super.key, required this.length, required this.filled, this.error = false});

  final int length;
  final int filled;
  final bool error;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    final on = error ? c.danger : c.primary;
    return Semantics(
      label: '$filled of $length digits entered',
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          for (var i = 0; i < length; i++)
            Container(
              width: 18,
              height: 18,
              margin: const EdgeInsets.symmetric(horizontal: 8),
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: i < filled ? on : Colors.transparent,
                border: Border.all(color: i < filled ? on : c.border, width: 2),
              ),
            ),
        ],
      ),
    );
  }
}

/// Section heading used inside panels.
class PosHeading extends StatelessWidget {
  const PosHeading(this.text, {super.key, this.subtitle, this.trailing});

  final String text;
  final String? subtitle;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Text(text, style: _text(context, PosType.title)),
        if (subtitle != null) ...[
          const SizedBox(width: PosSpacing.sm + 2),
          Text(subtitle!, style: _text(context, PosType.caption, color: c.textMuted)),
        ],
        const Spacer(),
        ?trailing,
      ],
    );
  }
}

/// Text in one of the type scale's styles, with the bundled font.
class PosText extends StatelessWidget {
  const PosText(
    this.text, {
    super.key,
    this.style = PosType.body,
    this.color,
    this.weight,
    this.size,
    this.align,
    this.maxLines,
  });

  final String text;
  final PosTypeStyle style;
  final Color? color;
  final FontWeight? weight;
  final double? size;
  final TextAlign? align;
  final int? maxLines;

  @override
  Widget build(BuildContext context) => Text(
    text,
    textAlign: align,
    maxLines: maxLines,
    overflow: maxLines == null ? null : TextOverflow.ellipsis,
    style: _text(context, style, color: color).copyWith(fontWeight: weight, fontSize: size),
  );
}
