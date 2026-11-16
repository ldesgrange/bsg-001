# The Bluetooth protocol

The tool talks to the BSG-001 over Bluetooth Low Energy by writing short command frames and reading the replies.
This page describes the rules that every command shares.
The page of each feature says what its commands are for, and this page is the only one that shows their bytes.
Bytes are written here as two hexadecimal digits.

## Where commands and replies travel

The device exposes one custom service with three characteristics.

| Role | UUID | Use |
| --- | --- | --- |
| Service | `00001523-1212-efde-1523-785feabcd123` | Groups the three characteristics below. |
| Command | `00001524-1212-efde-1523-785feabcd123` | The tool writes command frames here and asks for a write response. |
| Measurement | `00001525-1212-efde-1523-785feabcd123` | The device notifies its replies to queries here. |
| Log | `00001526-1212-efde-1523-785feabcd123` | The device notifies its records here. |

The device never speaks first: it sends a notification only in answer to a command.

## Command frames

A command frame is laid out as follows.

| Byte | Content |
| --- | --- |
| 0 | The opcode, which identifies the operation. |
| 1 | The payload length in bytes. |
| 2 and after | The payload. |

A command without a payload is a query, and is written as its opcode alone.

The tool builds the frame for an opcode and a payload as shown.

| opcode | payload | frame |
| --- | --- | --- |
| 30 | 04 41 42 43 44 | 30 05 04 41 42 43 44 |
| 30 | 01 5a | 30 02 01 5a |
| 31 | | 31 |

## Replies

A reply is laid out as follows.

| Byte | Content |
| --- | --- |
| 0 | The opcode of the query being answered. |
| 1 | A length byte that the device does not fill in reliably. |
| 2 and after | The content, then padding up to 20 bytes. |

Byte 1 looks like a content length, but it cannot be used as one.
The device sends `0b` there whatever the content, and `00` straight after power-up even though the content is present.
The padding is not always zero either: it can hold bytes left over from an earlier reply.

The tool therefore ignores byte 1 and keeps everything after it.
Each feature knows what it asked for, reads that from the start of what was kept, and ignores the rest.
A reply to an earlier query can arrive late, so the tool skips notifications whose opcode is not the one it is waiting for.

The tool takes the content of the reply to a query as shown.
The last column says why.

| query | reply | content | why |
| --- | --- | --- | --- |
| 31 | 31 0b 04 41 42 43 44 07 01 | 04 41 42 43 44 07 01 | Everything after byte 1 is kept; the feature will read `04 41 42 43 44` and ignore the leftover `07 01`. |
| 31 | 31 00 04 41 42 43 44 | 04 41 42 43 44 | Byte 1 says `00`, but the content is there and is kept. |
| 31 | 21 0b 0a 01 | rejected | Byte 0 is `21`, so this reply answers another query. |
| 31 | 31 | rejected | The reply stops before byte 1, so it is not a complete reply. |
| 31 | | rejected | The reply is empty. |

## Text

Text is sent as ASCII characters, preceded by one byte that counts them.

The tool turns text into bytes as shown.

| text | bytes |
| --- | --- |
| ABCD | 04 41 42 43 44 |
| A1b2 | 04 41 31 62 32 |
| Z | 01 5a |

Reading works the other way round, and stops after the counted characters.

The tool reads text from the start of some content as shown.

| content | text |
| --- | --- |
| 04 41 42 43 44 | ABCD |
| 04 41 42 43 44 07 01 02 | ABCD |
| 02 41 42 43 44 | AB |
| 04 41 42 | rejected |
| 04 41 42 43 ff | rejected |
| | rejected |

Text is rejected when the content stops before the counted characters or holds a byte that is not ASCII.

## Numbers

A number wider than one byte is sent least significant byte first.

The tool reads the number held in two bytes as shown.

| bytes | number |
| --- | --- |
| 05 00 | 5 |
| 00 01 | 256 |
| 6e 05 | 1390 |

## What each operation writes

Each operation of the tool writes the frames shown, in order.
It is 2026-01-02 03:04:05 in UTC when they run.

| operation | frames |
| --- | --- |
| set-name ABCD | 30 05 04 41 42 43 44, 31 |
| read | a0 |
| logging | 21 |
| records | 21, a1 |
| start --days 10 | a0, 21, 20 06 1a 01 02 03 04 05, 10 02 01 0a, a0, 21 |
| start --days 31 | a0, 21, 20 06 1a 01 02 03 04 05, 10 02 01 1f, a0, 21 |
| start --no-logging | a0, 21, 20 06 1a 01 02 03 04 05, 10 02 01 00, a0, 21 |
| stop | a0, 21, 10 02 00 0a, a0 |

Opcode `30` sets the name and has no reply, so the tool follows it with query `31`, which asks for the stored name.
Query `a0` asks for the current reading.
Query `21` asks for the logging state.
Query `a1` asks for the records, and the tool sends it only after `21` has told how many there are.

Opcode `20` sets the device's clock and has no reply.
Its payload is six bytes: the year since 2000, the month, the day, the hour, the minute and the second.
The tool sends it just before starting a run, because the device stamps the start of the run with its clock.
The time it sends is the one the computer shows at that moment, not the one it showed before connecting.
The bytes carry no time zone, and the tool sends the time in UTC.

The tool sends each time as the payload shown.

| time | payload |
| --- | --- |
| 2026-01-02 03:04:05 | 1a 01 02 03 04 05 |
| 2026-12-31 23:59:59 | 1a 0c 1f 17 3b 3b |
| 2000-01-01 00:00:00 | 00 01 01 00 00 00 |

Opcode `10` starts or stops a run and has no reply.
Its payload is two bytes: `01` to start or `00` to stop, then the logging duration in days, where `00` means measuring without recording.
The tool asks for the reading and the logging state first, to know what the device is doing, and asks again afterwards to check that it changed.
The vendor application sends the same frame to start and to stop, changing only the first payload byte, so a stop carries a logging duration too.
The device keeps the logging duration it is sent even when it is told to stop: a stop carrying `00` leaves it with logging off.
When stopping, the tool therefore sends the logging duration the device already has, so that stopping changes nothing else.

## The reading reply

The content of the reply to query `a0` is laid out as follows.

| Content byte | Holds |
| --- | --- |
| 0 | The status: `01` when measuring, `00` when stopped. |
| 1 | The days of the measuring time. |
| 2 | The hours of the measuring time. |
| 3 | The minutes of the measuring time. |
| 4 | The seconds of the measuring time. |
| 5 and 6 | The average dose rate, in hundredths of a microsievert per hour. |
| 7 and 8 | The dose rate of the last completed minute, in hundredths of a microsievert per hour, but ten times too low. |
| 9 and 10 | The battery sensor value. |

From each reply to the reading query, the tool reads the status, measuring time, average dose rate, last-minute dose rate¹ and battery sensor value shown.

| reply | status | measuring time | average dose rate | last-minute dose rate¹ | battery sensor |
| --- | --- | --- | --- | --- | --- |
| a0 0b 01 01 02 03 04 7b 00 05 00 d3 00 | measuring | 1 d 02:03:04 | 1.23 | 0.50 (1 pulse) | 211 |
| a0 00 00 00 00 00 00 00 00 00 00 63 00 07 01 | stopped | 0 d 00:00:00 | 0.00 | 0.00 (0 pulses) | 99 |
| a0 0b 01 0c 17 3b 3b 6e 05 00 00 c6 00 | measuring | 12 d 23:59:59 | 13.90 | 0.00 (0 pulses) | 198 |
| a0 0b 01 00 00 0b 00 6e 05 6e 05 c6 00 | measuring | 0 d 00:11:00 | 13.90 | 139.00 (278 pulses) | 198 |
| a0 0b 01 00 00 0b 00 31 00 31 00 c6 00 | measuring | 0 d 00:11:00 | 0.49 | 5.00 (10 pulses) | 198 |
| a0 0b 01 00 00 0b 00 6e 05 ff ff c6 00 | measuring | 0 d 00:11:00 | 13.90 | 6553.50 (13107 pulses) | 198 |

Both dose rates come from pulses that the device counts minute by minute, and it gives every pulse the same weight: `05`, that is 0.05 µSv/h.
That weight is right for the average, which spreads its pulses over ten minutes.
(¹) It is ten times too low for the last minute, where one pulse amounts to 0.50 µSv/h.

The tool therefore does not show bytes 7 and 8 as a dose rate.
It divides their value by five to get the pulses of the last minute, and counts each pulse as 0.50 µSv/h.
So `05 00` is 1 pulse, and `6e 05`, which is 1390, is 278 pulses.
A value that is not a multiple of five is rounded to the nearest pulse.
The largest value two bytes can hold, `ff ff`, which is 65535, is 13107 pulses.

A reply whose content is shorter than eleven bytes, or whose status is neither `00` nor `01`, is refused as invalid.

## The logging state reply

The content of the reply to query `21` is laid out as follows.

| Content byte | Holds |
| --- | --- |
| 0 | The logging duration in days, or `00` when logging is off. |
| 1 to 6 | The start of the run: year since 2000, month, day, hour, minute, second. |
| 7 | The record interval in minutes, where `00` stands for 10 minutes. |
| 8 and 9 | The number of records. |
| 10 | The checksum of the records: the sum of all their bytes, modulo 256. |

A start whose bytes do not make a real date and time, such as month `00`, means the clock was not set.

From each reply to the logging state query, the tool reads the logging duration, run started, record interval, records and checksum shown.

| reply | logging duration | run started | record interval | records | checksum |
| --- | --- | --- | --- | --- | --- |
| 21 0b 0a 1a 01 02 03 04 05 0a 90 00 73 | 10 days | 2026-01-02 03:04:05 | 10 minutes | 144 | 73 |
| 21 0b 0b 1a 0c 1f 17 3b 3b 3c 08 01 ff | 11 days | 2026-12-31 23:59:59 | 60 minutes | 264 | ff |
| 21 00 01 00 00 00 00 00 00 00 00 00 00 | 1 day | unknown | 10 minutes | 0 | 00 |
| 21 0b 00 1a 01 02 03 04 05 0a 00 00 00 07 01 | off | 2026-01-02 03:04:05 | 10 minutes | 0 | 00 |

A reply whose content is shorter than eleven bytes is refused as invalid.

## The records

The device answers query `a1` on the log characteristic, not on the measurement characteristic.
The answer does not follow the layout of a reply: it has no opcode and no length byte.
It is a run of packets of 20 bytes, to be put end to end.

Each record takes two bytes: its dose rate, in hundredths of a microsievert per hour.
The first record starts at the first byte of the first packet, and the last packet is padded with `00` up to 20 bytes.
The padding cannot be told from records of 0.00, so the number of records has to come from the logging state.

Given the number of records, the tool reads from the packets the records shown.

| number | packets | records |
| --- | --- | --- |
| 3 | 05 00 00 00 0a 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 | 0.05, 0.00, 0.10 |
| 1 | 6e 05 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 | 13.90 |
| 11 | 05 00 0a 00 00 00 0f 00 05 00 00 00 14 00 05 00 0a 00 00 00, 19 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 | 0.05, 0.10, 0.00, 0.15, 0.05, 0.00, 0.20, 0.05, 0.10, 0.00, 0.25 |
| 0 | | none |
| 2 | 05 00 | rejected |

Packets that hold fewer records than the logging state announced are rejected.

The checksum in the logging state is the sum of every byte of the records, modulo 256.

The tool works out the checksum of the records shown.

| records | checksum |
| --- | --- |
| 1.23 | 7b |
| 0.05, 0.00, 0.10 | 0f |
| 2.56, 2.55 | 00 |
| none | 00 |

The tool adds the bytes up as the packets arrive, and compares the result with the checksum in the logging state.
When they differ, it asks for the logging state and the records again, up to three times in all.

## Hazardous commands

Opcode `30` written without a payload can corrupt the stored name.
Opcode `40` puts the device in firmware-update mode until it is power-cycled.
The tool sends neither.
