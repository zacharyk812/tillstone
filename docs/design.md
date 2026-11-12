# Screen design (M1)

The first set of screen designs lives in the app as real Flutter screens with sample data. Open the app to see the **Screen designs** gallery, or look at the golden images in [`app/test/goldens/`](../app/test/goldens/) (light and dark, 1280 × 800).

These are a starting point, based on conventions that most counter-service, retail and restaurant point-of-sale systems share, so staff who've used any POS feel at home. Everything can be adjusted.

## Conventions

| Convention | Why |
|---|---|
| **Order on the right, products on the left** (register screens) | The layout almost every POS uses; the cashier's eye moves from choosing to reviewing to paying. |
| **Pay is always the biggest button**, bottom right | The most common action, and the one with the biggest consequence, is always in the same place. |
| **Search / scan bar first** | Barcode scanners type into it; in retail it's the main way to add items. |
| **Category tabs above a grid of large tiles** | Fast tapping on a touchscreen; color stripes help recognition without relying on color alone (names are always shown). |
| **Selling-area switcher in the top bar** for hybrids | One device can serve the café and the bookshop (PLAN §5). |
| **Sync status always visible** | Staff and managers can tell at a glance whether devices are in touch (PLAN §6). |
| **Tables colored by status** on the floor plan, with time seated | The standard way hosts and servers see the room. A legend is always shown, and status is also written as text. |
| **Kitchen display in dark theme**, oldest ticket first, timers green → amber → red, allergies in red | Readable from a distance in a bright kitchen; urgent tickets stand out. |
| **In-app keypads** for PINs, quantities and cash | The OS keyboard never appears on register screens (PLAN §11). |
| **48 px minimum tap targets, 64 px for primary actions** | Touch-first, fast and forgiving. |
| **Managers' tools in a separate Manage section** with a sidebar | Day-to-day screens stay uncluttered; permissions gate the section. |

## Screens in this set

| Screen | Shows |
|---|---|
| Sign in | PIN keypad, business and device name, sync status, training-mode notice |
| Register · quick service | Item grid with counts and sold-out state, modifiers on lines, dining option, discount line, Send and Pay |
| Register · retail | Scan-first search, stock on tiles, returns, customer lookup, a café item on a bookshop sale (hybrid) |
| Floor plan · full service | Tables by status, the selected table's check, seats and courses, a gift-shop item on a dinner check |
| Bar · tabs | Open tabs by name, card holds about to expire, quick drink grid |
| Payment | Tenders, quick-cash amounts, cash keypad, change due, card reader status |
| Kitchen display | Tickets with timers, modifiers and allergies, recalled tickets, bump |
| Manage · reports | Sales by selling area and tender, tips, refunds, loss-prevention summary |

## Not designed yet

Handheld (phone-size) order screen, customer display, order board, setup wizard, add-device flow, modifier picker sheet, split-check screen, shift close, catalog and staff editors, the Doctor screen, and the review list. These come with the milestones that need them.
