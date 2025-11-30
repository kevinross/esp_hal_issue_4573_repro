# To reproduce esp-rs/esp-hal#4573

## Setup

1. Clone esp-hal as a sibling to this repo (cargo refers to, eg esp-radio, as "../esp-hal/esp-radio")
2. Add the attribute specified in esp-hal.patch
3. Clone esp-mbedtls as a sibling to this repo (same idea as above)
4. apply the patch in mbedtls.patch to esp-mbedtls to make it compatible with esp-radio

## Reproducing

It seems the key lines that affect whether the crash happens are the log lines in `main.rs`: when there are enough of them, the crash triggers. The more you add, the more reliable the crash.
