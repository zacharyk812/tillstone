# Business templates

The setup wizard offers one choice per file in this folder. A template sets
up the selling areas and, optionally, sample items to edit later.

**Forks:** add a file here (for example `my-bakery.json`) and it appears in
the wizard on the next build. No code changes are needed. Keep your own
files under new names so upstream updates to these files don't conflict.

```jsonc
{
  "name": "Café or coffee shop",           // shown in the wizard
  "description": "One line about it.",
  "sort": 10,                              // lower comes first
  "areas": [{ "name": "Café", "mode": "quick_service" }],
                                           // mode: retail, quick_service, full_service, bar
  "option_groups": [                       // add-ons; prices are plain numbers
    { "name": "Milk", "min": 0, "max": 1, "options": [["Oat milk", "0.75"]] }
  ],
  "items": [
    {
      "name": "Latte", "area": "Café", "price": "4.75",
      "kind": "menu_item",                 // menu_item (default), product, service
      "sizes": [["Small", "4.25"], ["Large", "5.25"]],
      "options": ["Milk"],                 // option group names
      "stock": 12,                         // products only; starting stock
      "age_restricted": false
    }
  ],
  "staff": [{ "name": "Maya", "role": "cashier", "pin": "1111" }],
                                           // demo templates only
  "demo": { ... }                          // makes it a one-tap demo business
}
```

Prices are written in the business's currency and rounded to its minor
units (a template's "4.75" becomes ¥5 for a yen business). They're starting
points to edit, not real prices.

`cargo test -p pos-app` checks every template here, so a broken file fails
the build instead of the wizard.
