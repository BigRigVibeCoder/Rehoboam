"""Golden vectors for M4 (research/R12 §17-18): the Registry consumption basket and the famine price rule.

Basket (Allen 2001, Table 3):

Checks that `baskets.allen2001` in specs/manifest/registry_v0.toml reproduces the source's totals:
cost 414.899 g silver/yr at Strasbourg 1745-54 prices, 1,941 kcal/day and 80 g protein/day, and the
Mediterranean variant (oil for butter, wine for beer, 2 MBTU fuel) at 403.9 g and about 1,900 kcal/day
(our derivation from the source's prices; the paper does not print these two totals).
It also checks that every item maps to a Registry good, and that the M4 calorie floor
`subsistence_kg_grain` brackets the per-person floors Allen's numbers imply.

Price rule (Campbell & O Grada 2011, fn. 48): with constant elasticity and no storage or trade, a shortfall s multiplies
the price by (1 - s)^(-1/eps). eps = 0.403 reproduces Davenant's 17th-century schedule (s = 10..50% -> x1.3, 1.8, 2.6,
3.8, 5.5) within 7%; eps = 0.25 overshoots it 1.4-3x. The Registry range must contain the Davenant fit.

Run: python -I specs/economy/selftest_economy.py
"""
import sys
import tomllib
from pathlib import Path

REG = Path(__file__).resolve().parents[1] / "manifest" / "registry_v0.toml"
reg = tomllib.loads(REG.read_text(encoding="utf-8"))
b = reg["baskets"]["allen2001"]
items = {i["id"]: i for i in b["items"]}
subs = {i["id"]: i for i in b["substitutes"]}
goods = set(reg["goods"]["ids"])
fails = []

def check(name, got, want, tol):
    ok = abs(got - want) <= tol
    print(f"  {'ok  ' if ok else 'FAIL'} {name}: {got:.3f} (want {want} ± {tol})")
    if not ok:
        fails.append(name)

def totals(basket):
    cost = sum(i["qty"] * i["ref_price"] for i in basket)
    kcal = sum(i["qty"] * i["kcal"] for i in basket) / 365.0
    prot = sum(i["qty"] * i["protein_g"] for i in basket) / 365.0
    return cost, kcal, prot

print("Allen (2001) basket, northern variant")
cost, kcal, prot = totals(items.values())
check("cost g silver/yr", cost, 414.899, 0.005)
check("kcal/day", kcal, 1941.0, 1.0)
check("protein g/day", prot, 80.0, 0.5)

print("Mediterranean variant")
med = []
for i in items.values():
    if "mediterranean" in i:
        med.append(subs[i["mediterranean"]])
    elif "qty_mediterranean" in i:
        med.append({**i, "qty": i["qty_mediterranean"]})
    else:
        med.append(i)
cost_m, kcal_m, _ = totals(med)
check("cost g silver/yr", cost_m, 403.9, 0.1)
check("kcal/day", kcal_m, 1900.0, 2.0)

print("Registry consistency")
for i in list(items.values()) + list(subs.values()):
    if i["good"] not in goods:
        fails.append(f"good {i['good']}")
        print(f"  FAIL item {i['id']} maps to unknown good {i['good']!r}")
print(f"  ok   every item maps to a Registry good" if not any(f.startswith("good") for f in fails) else "")
lo, hi = reg["systems"]["M4.economy"]["params"]["subsistence_kg_grain"]
# Per-person floors implied by Allen: bread-only 193, Sukhatme 195, 3 baskets / 4 persons 204, ICMR 278 (R12 §18).
for name, v in [("bread-only", 193.0), ("Sukhatme", 195.0), ("3 baskets / 4", 204.0), ("ICMR", 278.0)]:
    ok = lo <= v <= hi
    print(f"  {'ok  ' if ok else 'FAIL'} subsistence_kg_grain [{lo}, {hi}] contains {name} {v}")
    if not ok:
        fails.append(name)
ratio = (b["family_baskets"] * (1 + b["rent_markup"]))
check("WR denominator (baskets)", ratio, 3.15, 1e-9)

print("Price rule (Davenant schedule)")
davenant = {0.1: 1.3, 0.2: 1.8, 0.3: 2.6, 0.4: 3.8, 0.5: 5.5}
def multiple(s, eps):
    return (1.0 - s) ** (-1.0 / eps)
worst = max(abs(multiple(s, 0.403) / m - 1.0) for s, m in davenant.items())
check("eps 0.403 worst relative error vs Davenant", worst, 0.0, 0.07)
steep = min(multiple(s, 0.25) / m for s, m in davenant.items() if s >= 0.2)
ok = steep > 1.3
print(f"  {'ok  ' if ok else 'FAIL'} eps 0.25 overshoots Davenant by >= 1.3x for s >= 20% (min ratio {steep:.2f})")
if not ok:
    fails.append("eps 0.25 overshoot")
elo, ehi = reg["systems"]["M4.economy"]["params"]["food_price_elasticity"]
ok = elo <= 0.403 <= ehi and elo <= 0.183 and ehi >= 0.87
print(f"  {'ok  ' if ok else 'FAIL'} food_price_elasticity [{elo}, {ehi}] contains Davenant 0.403 and the observed 0.183-0.87")
if not ok:
    fails.append("elasticity range")

print("M4 GOLDEN VECTORS:", "PASS" if not fails else f"FAIL {fails}")
sys.exit(1 if fails else 0)
