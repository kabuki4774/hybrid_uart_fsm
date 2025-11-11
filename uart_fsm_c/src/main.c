/**
 * @file main.c
 * @brief Main entry point for UART FSM test harness.
 * This file contains the main function that allows running
 * various test harness modes, including demo sequences,
 * reading from stdin, or reading from a serial device.
 *
 * Features:
 * - Command-line options for different modes
 * - Integration with harness functions
 * - Default demo execution
 *
 * Usage:
 * - `./uart_fsm_demo --test` to run built-in demos
 * - `./uart_fsm_demo --stdin` to read raw bytes from stdin
 * - `./uart_fsm_demo --serial <device>` to read from a serial device
 *
 * @author Nick Constant + ChatGPT
 * @date 2024-06-15
 */

#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "harness.h" /* include harness functions - simple single translation unit usage */

int main(int argc, char **argv)
{
    if (argc >= 2)
    {
        if (strcmp(argv[1], "--test") == 0)
        {
            run_demos();
            return 0;
        }
        else if (strcmp(argv[1], "--stdin") == 0)
        {
            run_from_stdin();
            return 0;
        }
        else if (strcmp(argv[1], "--serial") == 0 && argc >= 3)
        {
            run_from_serial(argv[2]);
            return 0;
        }
        else
        {
            fprintf(stderr, "Unknown option: %s\n", argv[1]);
            fprintf(stderr, "Usage: %s [--test] [--stdin]\n", argv[0]);
            return 1;
        }
    }

    /* Default: run demos */
    run_demos();
    return 0;
}