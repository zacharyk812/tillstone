import 'package:material_ui/material_ui.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

import '../main.dart' show ProofScreen;
import 'bar_tabs.dart';
import 'floor_plan.dart';
import 'kitchen_display.dart';
import 'payment.dart';
import 'pin_login.dart';
import 'register_quick_service.dart';
import 'register_retail.dart';
import 'reports.dart';

/// Every M1 screen design, with sample data, so they can be reviewed on any
/// platform. Replaced by the real app flow in M3.
class ScreenGallery extends StatelessWidget {
  const ScreenGallery({super.key});

  static final screens = <(String, String, IconData, Widget Function())>[
    ('Sign in', 'Staff PIN on the in-app keypad', Icons.pin_outlined, () => const PinLogin(initialDigits: 2)),
    (
      'Register · quick service',
      'Café counter: grid, modifiers, dining option',
      Icons.local_cafe_outlined,
      () => const QuickServiceRegister(),
    ),
    (
      'Register · retail',
      'Scan first, stock on tiles, returns',
      Icons.storefront_outlined,
      () => const RetailRegister(),
    ),
    (
      'Floor plan · full service',
      'Tables by status, seat-aware check',
      Icons.table_restaurant_outlined,
      () => const FloorPlan(),
    ),
    ('Bar · tabs', 'Open tabs, card holds, quick drinks', Icons.local_bar_outlined, () => const BarTabs()),
    ('Payment', 'Tenders, quick cash, change due', Icons.payments_outlined, () => const PaymentScreen()),
    ('Kitchen display', 'Tickets, timers, bump', Icons.soup_kitchen_outlined, () => const KitchenDisplay()),
    ('Manage · reports', 'Sales, tenders, loss prevention', Icons.insights_outlined, () => const ReportsScreen()),
    ('M0 proof', 'Rust core and peer-to-peer self-test', Icons.science_outlined, () => const ProofScreen()),
  ];

  @override
  Widget build(BuildContext context) {
    final c = PosThemeColors.of(context);
    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(PosSpacing.xl),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const PosBrand(),
              const SizedBox(height: PosSpacing.lg),
              const PosText('Screen designs', style: PosType.display),
              const SizedBox(height: PosSpacing.xs),
              PosText('Milestone M1 · sample data, not connected yet', color: c.textMuted),
              const SizedBox(height: PosSpacing.xl),
              Expanded(
                child: LayoutBuilder(
                  builder: (context, box) {
                    final columns = (box.maxWidth / 320).floor().clamp(1, 4);
                    return GridView.count(
                      crossAxisCount: columns,
                      mainAxisSpacing: PosSpacing.md,
                      crossAxisSpacing: PosSpacing.md,
                      childAspectRatio: 2.6,
                      children: [
                        for (final (title, subtitle, icon, build) in screens)
                          Material(
                            color: c.surface,
                            shape: RoundedRectangleBorder(
                              borderRadius: BorderRadius.circular(PosRadius.md),
                              side: BorderSide(color: c.border),
                            ),
                            child: InkWell(
                              borderRadius: BorderRadius.circular(PosRadius.md),
                              onTap: () =>
                                  Navigator.of(context)
                                      .push(MaterialPageRoute<void>(builder: (_) => _WithBack(child: build()))),
                              child: Padding(
                                padding: const EdgeInsets.all(PosSpacing.lg),
                                child: Row(
                                  children: [
                                    Icon(icon, size: 32, color: c.primary),
                                    const SizedBox(width: PosSpacing.lg),
                                    Expanded(
                                      child: Column(
                                        mainAxisAlignment: MainAxisAlignment.center,
                                        crossAxisAlignment: CrossAxisAlignment.start,
                                        children: [
                                          PosText(title, style: PosType.label, maxLines: 1),
                                          const SizedBox(height: 2),
                                          PosText(subtitle, style: PosType.caption, color: c.textMuted, maxLines: 2),
                                        ],
                                      ),
                                    ),
                                  ],
                                ),
                              ),
                            ),
                          ),
                      ],
                    );
                  },
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Adds a small floating "back to gallery" button over a screen design.
class _WithBack extends StatelessWidget {
  const _WithBack({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        child,
        Positioned(
          left: PosSpacing.sm,
          bottom: PosSpacing.sm,
          child: FloatingActionButton.small(
            tooltip: 'Back to screen designs',
            onPressed: () => Navigator.of(context).pop(),
            child: const Icon(Icons.grid_view),
          ),
        ),
      ],
    );
  }
}
