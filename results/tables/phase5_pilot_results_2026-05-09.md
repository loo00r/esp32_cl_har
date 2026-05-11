# Phase 5 Pilot Results Tables

Scope: real-device pilot sanity check on ESP32 + MPU6050. The second segment was standing-like small movement, not full Walking.

## Resource And CL Overhead

| Mode | PRED rows | Labels | Train updates | Replay RAM | Inference mean | Head mean | Update mean | Update / inference |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| no_adapt | 45 | 0 | 0 | 0 | 172.51 ms | 105.8 us | - | - |
| fifo | 50 | 20 | 2 | 12.0 KiB | 172.42 ms | 98.0 us | 664.5 us | 0.385% |
| reservoir | 50 | 20 | 2 | 12.0 KiB | 172.27 ms | 86.4 us | 658.0 us | 0.382% |

## Segment-Level Pilot Summary

| Mode | Segment | Attempts | Accepted labels | Rows | Accepted rate | Pred counts |
| --- | --- | --- | --- | --- | --- | --- |
| no_adapt | sitting | 1-6 | Sitting | 6 | 100.0% | Sitting=6 |
| no_adapt | standing_like | 7-45 | Standing\|Upstairs | 39 | 12.8% | Sitting=34;Standing=5 |
| fifo | sitting | 1-15 | Sitting | 15 | 100.0% | Sitting=15 |
| fifo | standing_like | 16-50 | Standing\|Upstairs | 35 | 20.0% | Sitting=28;Standing=7 |
| reservoir | sitting | 1-16 | Sitting | 16 | 100.0% | Sitting=16 |
| reservoir | standing_like | 17-50 | Standing\|Upstairs | 34 | 85.3% | Sitting=5;Standing=17;Upstairs=12 |

## Interpretation Notes

- These tables are pilot evidence, not final 6-class HAR accuracy.
- Persistence is off; CL state is RAM-only.
- FIFO and reservoir use the same replay budget: 16 slots/class, feature_dim=32.
- CL update overhead stays below 1% of MicroFlow-32 inference time in this pilot.
- Reservoir shows the strongest prediction distribution shift on the standing-like segment.

