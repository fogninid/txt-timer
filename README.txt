Pipe through standard input while highlighting and keeping track of delays between lines.

When completed print summary of maximum delays

Usage: txt-timer [OPTIONS]

Options:
  -q, --quiet
          do not output stdin

  -c, --count <COUNT>
          number of top differences to print at the end
          
          [default: 5]

  -B, --lines-before <LINES_BEFORE>
          [default: 5]

      --color-range <COLOR_RANGE>
          range for color scale of delay, in seconds
          
          [default: 0.2]

      --time-regex <TIME_REGEX>
          use regex to extract timestamp from lines instead of using real time, must have one (?<time> ) named capturing group

      --time-regex-format <TIME_REGEX_FORMAT>
          format of timestamp, without timezone see `strftime`. Example `%Y-%m-%d %H:%M:%S%.3f`

  -p, --prepend-time
          prepend time to output

  -o, --output-maximals <OUTPUT_MAXIMALS>
          redirect output of maximum differences to a file

  -h, --help
          Print help (see a summary with '-h')
