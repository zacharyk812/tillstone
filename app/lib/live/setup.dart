import 'package:flutter/services.dart' show FilteringTextInputFormatter;
import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

import '../src/rust/api/register.dart';
import '../src/session.dart';
import '../src/vault.dart';

/// Sets up a real business on this device (PLAN §11, §9): the business and
/// its kind, money and tax, the owner, and the owner passphrase. The
/// recovery kit follows. Every answer is checked again in Rust.
class SetupWizard extends StatefulWidget {
  const SetupWizard({super.key, required this.vault, required this.onCreated});

  final KeyVault vault;
  final ValueChanged<Session> onCreated;

  @override
  State<SetupWizard> createState() => _SetupWizardState();
}

class _SetupWizardState extends State<SetupWizard> {
  static const _steps = ['Your business', 'Money and tax', 'You', 'Owner passphrase'];

  final _templates = templates().where((t) => !t.demo).toList();
  final _currencies = currencies();

  final _name = TextEditingController();
  final _taxName = TextEditingController(text: 'Sales tax');
  final _taxPercent = TextEditingController();
  final _owner = TextEditingController();
  final _pin = TextEditingController();
  final _pin2 = TextEditingController();
  final _pass = TextEditingController();
  final _pass2 = TextEditingController();

  int _step = 0;
  late String _template = _templates.first.id;
  bool _samples = true;
  String _currency = 'USD';
  bool _tax = true;
  bool _taxIncluded = false;
  bool _creating = false;
  String? _createError;
  final _errors = <String, String?>{};

  @override
  void dispose() {
    for (final c in [_name, _taxName, _taxPercent, _owner, _pin, _pin2, _pass, _pass2]) {
      c.dispose();
    }
    super.dispose();
  }

  TemplateView get _chosen => _templates.firstWhere((t) => t.id == _template);

  /// Checks the current step; returns true if it's complete.
  bool _validate() {
    _errors.clear();
    switch (_step) {
      case 0:
        if (_name.text.trim().isEmpty) _errors['name'] = 'Enter your business name';
      case 1:
        if (_tax) {
          if (_taxName.text.trim().isEmpty) _errors['taxName'] = 'Name the tax, e.g. VAT or Sales tax';
          _errors['taxPercent'] = checkTaxPercent(percent: _taxPercent.text.trim());
        }
      case 2:
        if (_owner.text.trim().isEmpty) _errors['owner'] = 'Enter your name';
        _errors['pin'] = checkPin(pin: _pin.text);
        if (_errors['pin'] == null && _pin.text != _pin2.text) _errors['pin2'] = "The PINs don't match";
      case 3:
        _errors['pass'] = checkPassphrase(passphrase: _pass.text);
        if (_errors['pass'] == null && _pass.text != _pass2.text) _errors['pass2'] = "The passphrases don't match";
    }
    _errors.removeWhere((_, v) => v == null);
    setState(() {});
    return _errors.isEmpty;
  }

  Future<void> _next() async {
    if (!_validate()) return;
    if (_step < _steps.length - 1) {
      setState(() => _step++);
      return;
    }
    setState(() {
      _creating = true;
      _createError = null;
    });
    try {
      final session = await Session.create(
        widget.vault,
        SetupInput(
          businessName: _name.text.trim(),
          currency: _currency,
          templateId: _template,
          sampleItems: _samples && _chosen.itemCount > 0,
          taxName: _tax ? _taxName.text.trim() : null,
          taxPercent: _taxPercent.text.trim(),
          taxIncluded: _taxIncluded,
          ownerName: _owner.text.trim(),
          ownerPin: _pin.text,
          passphrase: _pass.text,
        ),
      );
      if (!mounted) return;
      widget.onCreated(session);
      Navigator.of(context).pop();
    } on Object catch (e) {
      if (!mounted) return;
      setState(() {
        _creating = false;
        _createError = 'Setup failed: $e';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return Scaffold(
      body: SafeArea(
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 640),
            child: Padding(
              padding: const EdgeInsets.all(PosSpacing.xl),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Row(
                    children: [
                      const PosBrand(),
                      const Spacer(),
                      PosText('Step ${_step + 1} of ${_steps.length}', color: c.textMuted),
                    ],
                  ),
                  const SizedBox(height: PosSpacing.lg),
                  ClipRRect(
                    borderRadius: BorderRadius.circular(PosRadius.sm),
                    child: LinearProgressIndicator(
                      value: (_step + 1) / _steps.length,
                      minHeight: 6,
                      color: c.primary,
                      backgroundColor: c.surfaceMuted,
                    ),
                  ),
                  const SizedBox(height: PosSpacing.xl),
                  PosText(_steps[_step], style: PosType.display),
                  const SizedBox(height: PosSpacing.lg),
                  Expanded(child: SingleChildScrollView(child: _body(c))),
                  if (_createError != null) ...[
                    const SizedBox(height: PosSpacing.md),
                    PosText(_createError!, color: c.danger, weight: FontWeight.w600),
                  ],
                  const SizedBox(height: PosSpacing.lg),
                  Row(
                    children: [
                      PosButton(
                        label: _step == 0 ? 'Cancel' : 'Back',
                        kind: PosButtonKind.secondary,
                        onPressed: _creating
                            ? null
                            : () => _step == 0 ? Navigator.of(context).pop() : setState(() => _step--),
                      ),
                      const Spacer(),
                      PosButton(
                        label: _creating
                            ? 'Creating…'
                            : _step == _steps.length - 1
                            ? 'Create business'
                            : 'Next',
                        large: true,
                        onPressed: _creating ? null : _next,
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _gap() => const SizedBox(height: PosSpacing.lg);

  Widget _body(PosColors c) {
    switch (_step) {
      case 0:
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            PosTextField(
              key: const ValueKey('setup-name'),
              label: 'Business name',
              hint: 'e.g. Harbor Street Coffee',
              controller: _name,
              error: _errors['name'],
              autofocus: true,
              maxLength: 80,
            ),
            _gap(),
            const PosText('What kind of business?', style: PosType.label),
            const SizedBox(height: PosSpacing.sm),
            for (final t in _templates) ...[
              _Choice(
                key: ValueKey('template-${t.id}'),
                title: t.name,
                subtitle: t.description,
                selected: t.id == _template,
                onTap: () => setState(() => _template = t.id),
              ),
              const SizedBox(height: PosSpacing.sm),
            ],
            if (_chosen.itemCount > 0)
              _Toggle(
                label: 'Start with ${_chosen.itemCount} sample items to edit',
                value: _samples,
                onChanged: (v) => setState(() => _samples = v),
              ),
            PosText(
              'You can add more selling areas, like a shop next to a café, later in Manage.',
              style: PosType.caption,
              color: c.textMuted,
            ),
          ],
        );
      case 1:
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const PosText('Currency', style: PosType.label),
            const SizedBox(height: PosSpacing.xs),
            DropdownButtonFormField<String>(
              key: const ValueKey('setup-currency'),
              initialValue: _currency,
              isExpanded: true,
              decoration: InputDecoration(
                filled: true,
                fillColor: c.surface,
                border: OutlineInputBorder(borderRadius: BorderRadius.circular(PosRadius.md)),
              ),
              items: [
                for (final cur in _currencies)
                  DropdownMenuItem(value: cur.code, child: Text('${cur.code} · ${cur.name}')),
              ],
              onChanged: (v) => setState(() => _currency = v ?? _currency),
            ),
            const SizedBox(height: PosSpacing.xs),
            PosText("This can't be changed later.", style: PosType.caption, color: c.textMuted),
            _gap(),
            _Toggle(label: 'Charge a tax on sales', value: _tax, onChanged: (v) => setState(() => _tax = v)),
            if (_tax) ...[
              const SizedBox(height: PosSpacing.sm),
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    flex: 3,
                    child: PosTextField(
                      key: const ValueKey('setup-tax-name'),
                      label: 'Tax name',
                      controller: _taxName,
                      error: _errors['taxName'],
                    ),
                  ),
                  const SizedBox(width: PosSpacing.md),
                  Expanded(
                    flex: 2,
                    child: PosTextField(
                      key: const ValueKey('setup-tax-percent'),
                      label: 'Rate (%)',
                      hint: 'e.g. 8.25',
                      controller: _taxPercent,
                      error: _errors['taxPercent'],
                      keyboardType: const TextInputType.numberWithOptions(decimal: true),
                      inputFormatters: [FilteringTextInputFormatter.allow(RegExp(r'[0-9.]'))],
                    ),
                  ),
                ],
              ),
              const SizedBox(height: PosSpacing.sm),
              _Toggle(
                label: 'Prices already include this tax',
                value: _taxIncluded,
                onChanged: (v) => setState(() => _taxIncluded = v),
              ),
              PosText(
                'Usually on in Europe and Australia (VAT, GST); off in the US and Canada.',
                style: PosType.caption,
                color: c.textMuted,
              ),
            ],
          ],
        );
      case 2:
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            PosTextField(
              key: const ValueKey('setup-owner'),
              label: 'Your name',
              controller: _owner,
              error: _errors['owner'],
              autofocus: true,
              maxLength: 60,
            ),
            _gap(),
            PosTextField(
              key: const ValueKey('setup-pin'),
              label: 'Your PIN',
              help: '4 to 8 digits. You sign in to the register with it.',
              controller: _pin,
              error: _errors['pin'],
              obscure: true,
              keyboardType: TextInputType.number,
              inputFormatters: [FilteringTextInputFormatter.digitsOnly],
              maxLength: 8,
            ),
            _gap(),
            PosTextField(
              key: const ValueKey('setup-pin2'),
              label: 'Your PIN again',
              controller: _pin2,
              error: _errors['pin2'],
              obscure: true,
              keyboardType: TextInputType.number,
              inputFormatters: [FilteringTextInputFormatter.digitsOnly],
              maxLength: 8,
            ),
          ],
        );
      default:
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            PosText(
              'The owner passphrase protects the things only an owner can do: adding registers, '
              'removing them, and changing who belongs to the business. It is never stored; '
              'a stolen register is useless for these without it.',
              color: c.textMuted,
            ),
            _gap(),
            PosTextField(
              key: const ValueKey('setup-pass'),
              label: 'Owner passphrase',
              help: 'At least 10 characters. A few unrelated words work well.',
              controller: _pass,
              error: _errors['pass'],
              obscure: true,
              autofocus: true,
            ),
            _gap(),
            PosTextField(
              key: const ValueKey('setup-pass2'),
              label: 'Owner passphrase again',
              controller: _pass2,
              error: _errors['pass2'],
              obscure: true,
              onSubmitted: (_) => _next(),
            ),
            _gap(),
            PosText(
              'Next you will get a recovery code to write down, in case you forget the passphrase '
              'or lose every owner device.',
              style: PosType.caption,
              color: c.textMuted,
            ),
          ],
        );
    }
  }
}

class _Choice extends StatelessWidget {
  const _Choice({super.key, required this.title, required this.subtitle, required this.selected, required this.onTap});

  final String title;
  final String subtitle;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return Semantics(
      selected: selected,
      button: true,
      child: Material(
        color: selected ? c.primary.withValues(alpha: 0.08) : c.surface,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(PosRadius.md),
          side: BorderSide(color: selected ? c.primary : c.border, width: selected ? 2 : 1),
        ),
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(PosRadius.md),
          child: Padding(
            padding: const EdgeInsets.all(PosSpacing.md),
            child: Row(
              children: [
                Icon(
                  selected ? Icons.radio_button_checked : Icons.radio_button_unchecked,
                  color: selected ? c.primary : c.textMuted,
                ),
                const SizedBox(width: PosSpacing.md),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      PosText(title, style: PosType.label),
                      PosText(subtitle, style: PosType.caption, color: c.textMuted),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _Toggle extends StatelessWidget {
  const _Toggle({required this.label, required this.value, required this.onChanged});

  final String label;
  final bool value;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) => MergeSemantics(
    child: InkWell(
      onTap: () => onChanged(!value),
      borderRadius: BorderRadius.circular(PosRadius.md),
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: PosSize.minTapTarget),
        child: Row(
          children: [
            Expanded(child: PosText(label)),
            Switch(value: value, onChanged: onChanged),
          ],
        ),
      ),
    ),
  );
}
