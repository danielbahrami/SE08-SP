# Smart Lock

## LED Setup
- Red = GPIO25
- Green = GPIO32
- Blue = GPIO33

## LED Indicaton
- Blue = INITIALIZING (Connecting to Wi-Fi and MQTT)
- Red = LOCKED
    - Blinking = ERROR
- Yellow = LOCKING/UNLOCKING
- Green = UNLOCKED

## Commands
- unlock
    - => UNLOCKING => UNLOCKED
- lock
    - => LOCKING => LOCKED
