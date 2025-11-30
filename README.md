# To reproduce esp-rs/esp-hal#4573

It seems the key lines that affect whether the crash happens are the log lines in `main.rs`: when there are enough of them, the crash triggers. The more you add, the more reliable the crash.
