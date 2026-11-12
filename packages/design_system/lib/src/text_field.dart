import 'package:flutter/services.dart' show TextInputFormatter;
import 'package:material_ui/material_ui.dart';

import 'theme.dart';
import 'tokens.g.dart';

/// A labelled text field for setup and Manage screens. Register screens use
/// the in-app numpad instead, so the OS keyboard never covers a sale.
class PosTextField extends StatelessWidget {
  const PosTextField({
    super.key,
    required this.label,
    this.controller,
    this.hint,
    this.help,
    this.error,
    this.obscure = false,
    this.keyboardType,
    this.inputFormatters,
    this.onChanged,
    this.onSubmitted,
    this.autofocus = false,
    this.maxLength,
  });

  final String label;
  final TextEditingController? controller;
  final String? hint;

  /// Shown under the field when there's no error.
  final String? help;
  final String? error;
  final bool obscure;
  final TextInputType? keyboardType;
  final List<TextInputFormatter>? inputFormatters;
  final ValueChanged<String>? onChanged;
  final ValueChanged<String>? onSubmitted;
  final bool autofocus;
  final int? maxLength;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    OutlineInputBorder border(Color color, [double width = 1]) => OutlineInputBorder(
      borderRadius: BorderRadius.circular(PosRadius.md),
      borderSide: BorderSide(color: color, width: width),
    );
    final caption = TextStyle(
      fontFamily: PosTheme.fontFamily,
      package: PosTheme.fontPackage,
      fontSize: PosType.caption.size,
      height: PosType.caption.height,
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          label,
          style: TextStyle(
            fontFamily: PosTheme.fontFamily,
            package: PosTheme.fontPackage,
            fontSize: PosType.label.size,
            fontWeight: PosType.label.weight,
            color: c.text,
          ),
        ),
        const SizedBox(height: PosSpacing.xs),
        TextField(
          controller: controller,
          obscureText: obscure,
          autocorrect: !obscure,
          enableSuggestions: !obscure,
          keyboardType: keyboardType,
          inputFormatters: inputFormatters,
          onChanged: onChanged,
          onSubmitted: onSubmitted,
          autofocus: autofocus,
          maxLength: maxLength,
          style: TextStyle(
            fontFamily: PosTheme.fontFamily,
            package: PosTheme.fontPackage,
            fontSize: PosType.body.size,
            color: c.text,
          ),
          decoration: InputDecoration(
            hintText: hint,
            hintStyle: TextStyle(color: c.textMuted),
            filled: true,
            fillColor: c.surface,
            counterText: '',
            contentPadding: const EdgeInsets.symmetric(horizontal: PosSpacing.md, vertical: PosSpacing.md),
            constraints: const BoxConstraints(minHeight: PosSize.minTapTarget),
            enabledBorder: border(error == null ? c.border : c.danger),
            focusedBorder: border(error == null ? c.primary : c.danger, 2),
            border: border(c.border),
          ),
        ),
        if (error != null || help != null) ...[
          const SizedBox(height: PosSpacing.xs),
          Text(
            error ?? help!,
            style: caption.copyWith(
              color: error == null ? c.textMuted : c.danger,
              fontWeight: error == null ? null : FontWeight.w600,
            ),
          ),
        ],
      ],
    );
  }
}
