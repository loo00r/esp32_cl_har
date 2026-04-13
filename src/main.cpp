#include <Arduino.h>

#define LED_PIN 2

static int count = 0;

void setup() {
    Serial.begin(115200);
    pinMode(LED_PIN, OUTPUT);
    Serial.println("ESP32 HAR started");
}

void loop() {
    if (count >= 50) return;

    digitalWrite(LED_PIN, HIGH);
    delay(500);
    digitalWrite(LED_PIN, LOW);
    delay(500);

    count++;
    Serial.printf("blink %d\n", count);

    if (count == 50) {
        Serial.printf("done after %d blinks\n", count);
    }
}
