import 'package:material_ui/material_ui.dart';

import 'theme.dart';
import 'tokens.g.dart';

enum PosButtonKind { primary, secondary, danger }

/// The one button used across Tillstone. Touch-first: at least 48px tall,
/// 64px for primary actions.
class PosButton extends StatelessWidget {
  const PosButton({
    super.key,
    required this.label,
    required this.onPressed,
    this.kind = PosButtonKind.primary,
    this.large = false,
  });

  final String label;
  final VoidCallback? onPressed;
  final PosButtonKind kind;

  /// Use for the main action on a screen, such as Pay.
  final bool large;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    final (background, foreground) = switch (kind) {
      PosButtonKind.primary => (c.primary, c.onPrimary),
      PosButtonKind.secondary => (c.surfaceMuted, c.text),
      PosButtonKind.danger => (c.danger, c.onDanger),
    };
    final enabled = onPressed != null;
    return Semantics(
      button: true,
      enabled: enabled,
      label: label,
      excludeSemantics: true,
      child: ConstrainedBox(
        constraints: BoxConstraints(
          minHeight: large ? PosSize.primaryTapTarget : PosSize.minTapTarget,
          minWidth: PosSize.minTapTarget,
        ),
        child: Material(
          color: enabled ? background : c.surfaceMuted,
          borderRadius: BorderRadius.circular(PosRadius.md),
          child: InkWell(
            onTap: onPressed,
            borderRadius: BorderRadius.circular(PosRadius.md),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: PosSpacing.lg, vertical: PosSpacing.md),
              child: Center(
                widthFactor: 1,
                child: Text(
                  label,
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    fontSize: PosType.label.size,
                    fontWeight: PosType.label.weight,
                    height: PosType.label.height,
                    color: enabled ? foreground : c.textMuted,
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
