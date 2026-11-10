"""Part (c) review: where each model of train.py wins and loses against the rules, file by file.

    python -I ml/review.py > ml/review.txt

Same signals, models and folds as train.py, over 11 splits (seeds 0 to 9 and train.py's own). Two
things train.py's totals don't show: which answers hold from split to split, and how the count looks
on the files where the answer changes what the app can do. Nothing is saved, nothing goes into the
app (docs/ARQUITETURA.md, stage 8).
"""
import collections
import csv
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))  # -I leaves the script's folder out

import numpy as np

import train as t

SEEDS = [*range(10), t.SEED]


def family(name):
    return "encceja-livro" if name.startswith("encceja-livro") else re.match(r"[a-z]+(-\d{4})?", name)[0]


def tally(rows, mask, suffix=None):
    names = (f"{family(r['file'])} {r['label']}{suffix[i] if suffix is not None else ''}" for i, r in enumerate(rows) if mask[i])
    return ", ".join(f"{k} x{v}" for k, v in sorted(collections.Counter(names).items())) or "none"


def main():
    with open(Path(__file__).with_name("signals.csv"), newline="", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))
    x = np.array([[np.log1p(float(r[c])) if c in t.LOGGED else float(r[c]) for c in t.FEATURES] for r in rows])
    y = np.array([r["label"] for r in rows], dtype=object)
    rules = np.array([r["rule_kind"] for r in rows], dtype=object)
    groups = np.array([t.edition(r["file"]) for r in rows], dtype=object)
    truth = t.kind(y)
    questions = np.array([int(r["choice_questions"]) + int(r["sum_questions"]) for r in rows])
    # Nothing to run: an exam or a handout where the reader finds no question (open exercises, a
    # scan), or an answer key with no answer read (an image). The right label changes nothing there.
    dead = ((truth != "content") & (truth != "key") & (questions == 0)) | ((truth == "key") & (rules != "key"))
    live = ~dead
    assert 0 < dead.sum() < len(rows) // 5, dead.sum()
    rules_right = rules == truth

    print(f"{len(rows)} files, {len(SEEDS)} splits by edition (seeds {SEEDS})")
    print(f"nothing for the app to run in {dead.sum()} files: {tally(rows, dead)}")
    print(f"rules: {rules_right.sum()} right; on the other {live.sum()} files, {rules_right[live].sum()}")
    for name, model in t.MODELS.items():
        # As in train.py: the key parser decides what is a key, the model the rest.
        right = np.array([np.where(rules == "key", "key", t.kind(t.out_of_fold(model, x, y, groups, s))) for s in SEEDS]) == truth
        hits = right.sum(axis=0)
        won, lost = (hits >= 9) & ~rules_right, (hits <= 2) & rules_right
        flaky = (hits > 2) & (hits < 9)
        mark = np.where(dead, " (nothing to run)", "")
        print(f"\n{name} + key parser")
        for title, where in (("all files", slice(None)), (f"the {live.sum()} files with something to run", live)):
            totals = right[:, where].sum(axis=1)
            base = rules_right[where].sum()
            print(f"  {title}: {totals.min()} to {totals.max()} right (rules {base});"
                  f" above the rules in {(totals > base).sum()} splits, level in {(totals == base).sum()}, below in {(totals < base).sum()}")
        print(f"  right in 9+ splits where the rules are wrong: {won.sum()} files, {(won & live).sum()} with something to run")
        print(f"    {tally(rows, won, mark)}")
        print(f"  wrong in 9+ splits where the rules are right: {lost.sum()} files, {(lost & live).sum()} with something to run")
        print(f"    {tally(rows, lost, mark)}")
        print(f"  answer changes with the split (right in 3 to 8): {flaky.sum()} files, {(flaky & rules_right).sum()} of them right by the rules")
        both = truth == "both"
        print(f"  both: {right[:, both].sum(axis=1).min()} to {right[:, both].sum(axis=1).max()} of {both.sum()} right (rules {rules_right[both].sum()})")


if __name__ == "__main__":
    main()
