#ifndef HARNESS_H
#define HARNESS_H

void demo_valid_sequence(int capture_mode);
void demo_noise_reset(int capture_mode);
void run_demos(void);
void run_from_stdin(void);
void run_from_serial(const char *device);

#endif /* HARNESS_H */
