"""Stage 8 (b): trains two small classifiers on ml/signals.csv and measures them next to the rules.

    python -I ml/train.py > ml/results.txt

Reads only ml/signals.csv (numbers and labels, no document) and writes nothing but the report.
Every file is predicted by a model that never saw its edition (the source-year prefix of the name).
"""
import csv
import re
from pathlib import Path

import numpy as np
import sklearn
from sklearn.base import clone
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import confusion_matrix, precision_recall_fscore_support
from sklearn.model_selection import StratifiedGroupKFold
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import StandardScaler
from sklearn.tree import DecisionTreeClassifier, export_text

SEED = 8
FOLDS = 5
FEATURES = ["choice_questions", "sum_questions", "headers", "exam_words", "chars", "density", "first_at"]
LOGGED = {"choice_questions", "sum_questions", "headers", "chars"}  # counts that span orders of magnitude
LABELS = ["content", "exam", "exam_sum", "both", "key"]
KINDS = ["content", "exam", "both", "key"]  # what the app knows: exam_sum is an exam

# Fixed before the first run, never tuned: the same files score the models, so tuning would be cheating.
MODELS = {
    "logistic": make_pipeline(StandardScaler(), LogisticRegression(class_weight="balanced", max_iter=2000)),
    "tree": DecisionTreeClassifier(max_depth=4, class_weight="balanced", random_state=SEED),
}


def edition(name):
    m = re.match(r"([a-z]+(?:-[a-z]+)*?)-(\d{4})", name)
    return f"{m[1]}-{m[2]}" if m else name.rsplit(".", 1)[0]


assert edition("enem-2023-d1-gabarito.pdf") == "enem-2023"
assert edition("uem-2020-p2-mat.pdf") == edition("uem-2020-gabarito-p2.pdf")
assert edition("wikibooks-calculo-volume-1-limites.txt") == "wikibooks-calculo-volume-1-limites"


def kind(labels):
    return np.where(labels == "exam_sum", "exam", labels)


def out_of_fold(model, x, y, groups, seed):
    """Each file's label as said by a model trained without that file's edition."""
    said = np.full(len(y), "", dtype=object)
    for train, test in StratifiedGroupKFold(FOLDS, shuffle=True, random_state=seed).split(x, y, groups):
        assert not set(groups[train]) & set(groups[test]), "an edition on both sides of the split"
        said[test] = clone(model).fit(x[train], y[train]).predict(x[test])
    assert (said != "").all()
    return said


def report(title, truth, said, names):
    table = confusion_matrix(truth, said, labels=names)
    precision, recall, _, _ = precision_recall_fscore_support(truth, said, labels=names, zero_division=0)
    print(f"\n{title}: {(truth == said).sum()} of {len(truth)} right")
    print(f"{'label / said':>13}" + "".join(f"{n:>9}" for n in names) + f"{'recall':>9}")
    for name, row, r in zip(names, table, recall):
        print(f"{name:>13}" + "".join(f"{v:>9}" for v in row) + f"{r:>9.2f}")
    print(f"{'precision':>13}" + "".join(f"{p:>9.2f}" for p in precision))


def main():
    with open(Path(__file__).with_name("signals.csv"), newline="", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))
    x = np.array([[np.log1p(float(r[c])) if c in LOGGED else float(r[c]) for c in FEATURES] for r in rows])
    y = np.array([r["label"] for r in rows], dtype=object)
    rules = np.array([r["rule_kind"] for r in rows], dtype=object)
    sure = np.array([r["rule_sure"] == "true" for r in rows])
    groups = np.array([edition(r["file"]) for r in rows], dtype=object)
    assert set(y) == set(LABELS) and set(rules) <= set(KINDS)

    print(f"{len(rows)} files, {len(set(groups))} editions, {FOLDS} folds by edition, seed {SEED}")
    print(f"scikit-learn {sklearn.__version__}, numpy {np.__version__}")
    truth = kind(y)
    rest = truth != "key"
    report("rules (the app today)", truth, rules, KINDS)
    print(f"  where the rules say sure: {(truth == rules)[sure].sum()} of {sure.sum()};"
          f" unsure: {(truth == rules)[~sure].sum()} of {(~sure).sum()}")
    print(f"  on the {rest.sum()} files that are not keys: {(truth == rules)[rest].sum()}")

    for name, model in MODELS.items():
        said = out_of_fold(model, x, y, groups, SEED)
        report(f"{name}, 5 labels", y, said, LABELS)
        # What part (c) would ship: the key parser still decides what is a key, the model the rest.
        mixed = np.where(rules == "key", "key", kind(said))
        report(f"{name} + key parser, as the app's 4 kinds", truth, mixed, KINDS)
        print(f"  without the key parser: {(kind(said) == truth).sum()} right")
        print(f"  on the {rest.sum()} files that are not keys: {(mixed == truth)[rest].sum()}")
        won, lost = ((mixed == truth) & (rules != truth)).sum(), ((mixed != truth) & (rules == truth)).sum()
        print(f"  against the rules: right where they are wrong in {won}, wrong where they are right in {lost}")
        for label, where in (("sure", sure), ("unsure", ~sure)):
            print(f"  where the rules say {label}: {(mixed == truth)[where].sum()} of {where.sum()}")
        # The split is luck too: the same measure over 10 other splits.
        spread = [
            np.where(rules == "key", "key", kind(out_of_fold(model, x, y, groups, s))) == truth for s in range(10)
        ]
        print(f"  over 10 other splits (seeds 0-9): {min(r.sum() for r in spread)} to {max(r.sum() for r in spread)}"
              f" right; not keys: {min(r[rest].sum() for r in spread)} to {max(r[rest].sum() for r in spread)}")

    # How the tree decides, trained on every file: no measure comes from this one.
    print("\ntree on all files (chars, headers and question counts are ln(1 + n)):")
    print(export_text(clone(MODELS["tree"]).fit(x, y), feature_names=FEATURES))


if __name__ == "__main__":
    main()
