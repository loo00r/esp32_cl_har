# esp32_cl_har

Continual Learning for Human Activity Recognition on ESP32 using C++ and TFLite Micro.

## Hardware

- **MCU**: ESP32-WROOM-32 (ESP32-D0WD-V3 rev3.1, 240 MHz, 320 KB SRAM, 4 MB Flash)
- **Sensor**: MPU6050 (GY-521) — SCL→GPIO22, SDA→GPIO21
- **Port**: `/dev/ttyUSB0`

## Prerequisites

### 1. PlatformIO

```bash
pip install platformio
```

### 2. Add user to dialout (USB port access)

```bash
sudo usermod -a -G dialout $USER
# log out and back in for it to take effect
```

## Build & Flash

```bash
pio run                        # compile only
pio run -t upload              # compile + flash
pio run -t upload && pio device monitor   # flash + open serial monitor
```

## Project Structure

```
esp32_cl_har/
├── src/
│   └── main.cpp               # entry point, main loop
├── include/                   # headers (model_data.h etc.)
├── platformio.ini             # board, framework, dependencies
├── model.tflite               # quantized 1D-CNN model
└── notebooks/                 # PC-side Python (training, export)
```

## Key Dependencies

| Library | Purpose |
|---------|---------|
| `tensorflow/TensorFlowLite_ESP32` | TFLite Micro inference engine |
| `electroniccats/MPU6050` | MPU6050 I2C driver |
