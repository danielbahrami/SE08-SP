# Lock-n-Roll

## Setup

- Red = GPIO25
- Green = GPIO32
- Blue = GPIO33

## LED indicator

- Blue = Initializing (connection to wifi and mqtt)
- Red = Locked
- Orange = Locking
- Green = Unlocked
- Yellow = Unlocking
- Blinking Red = Error

## Commands

See the command topic in [config.toml](./.cargo/config.toml)

- open: OPENING => OPEN
- close: CLOSING => CLOSED
- \<anything else\> => ERROR

## TODO

Model the states better
