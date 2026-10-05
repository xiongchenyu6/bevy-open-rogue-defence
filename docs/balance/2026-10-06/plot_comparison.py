"""Plot measured passive durations; dots are individual completed runs."""

import csv
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

root = Path(__file__).resolve().parent
with (root / "analysis/cases.csv").open() as handle:
    rows = list(csv.DictReader(handle))
groups = [
    ("warrior", "behind"),
    ("archer", "behind"),
    ("mage", "behind"),
    ("archer", "forward"),
    ("mage", "forward"),
]
figure, axis = plt.subplots(figsize=(10, 5))
for offset, batch, color in [(-0.18, "before", "#607DAB"), (0.18, "after", "#CB6447")]:
    samples = [
        [
            float(row["elapsed"]) / 60
            for row in rows
            if row["batch"] == batch
            and row["policy"] == "passive"
            and (row["starter_class"], row["position"]) == group
            and row["outcome"] == "loss"
        ]
        for group in groups
    ]
    assert all(len(sample) == 2 for sample in samples)
    x = np.arange(len(groups)) + offset
    axis.bar(
        x,
        [sum(s) / len(s) for s in samples],
        width=0.34,
        color=color,
        alpha=0.72,
        label=batch,
    )
    for position, values in zip(x, samples):
        axis.scatter(
            [position - 0.035, position + 0.035],
            values,
            color=color,
            edgecolors="white",
            zorder=3,
        )
axis.set_xticks(
    np.arange(len(groups)), [f"{kind.title()}\n{position}" for kind, position in groups]
)
axis.set_ylabel("Actual game minutes until final life lost")
axis.set_ylim(0, 26)
axis.grid(axis="y", alpha=0.2)
axis.set_axisbelow(True)
axis.legend(title="Measured rules")
axis.set_title(
    "No optional growth choices: pressure arrives earlier\nNormal chapter 1; two seeds per class/position; mandatory recruitment retained"
)
figure.tight_layout()
figure.savefig(root / "passive-duration.png", dpi=160)
figure.savefig(root / "passive-duration.svg")
plt.close(figure)
