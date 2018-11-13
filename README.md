

## Object Spam

A simple benchmarking tool with tunable parallelism, loads, and durations.  
Reports basic stats and histograms of read and write execution. 

This will use the standard AWS env vars such as `AWS_DEFAULT_PROFILE` and `AWS_*_ACCESS_KEY_*`


### example run:
```
$ object-spam

Creating test bucket object-spam-1542072797.526479000
Working ...
Error: parsed HTTP message from remote is incomplete
Error: parsed HTTP message from remote is incomplete
Error: The specified key does not exist.
Error: The specified key does not exist.
Done!
Cleaning up ...

Stats
Timing results for Read
Mean = 1.5442215
Max = 3.088443
Min = 0.135582
50th Percenile value = 1.5442215 | count = 18
70th Percenile value = 2.1619100999999996 | count = 5
90th Percenile value = 2.7795986999999998 | count = 2
95th Percenile value = 2.9340208499999996 | count = 1
99th Percenile value = 3.05755857 | count = 1
Timing results for Write
Mean = 4.3119065
Max = 8.623813
Min = 0.142206
50th Percenile value = 4.3119065 | count = 13
70th Percenile value = 6.0366691 | count = 7
90th Percenile value = 7.7614317 | count = 2
95th Percenile value = 8.19262235 | count = 2
99th Percenile value = 8.53757487 | count = 1

```

### Help
```


USAGE:
    object-spam [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -j, --json       Produce output in json instead of ascii prettyness
    -V, --version    Prints version information

OPTIONS:
        --aws_access_key_id <aws_access_key_id>            The AWS access key id [env: AWS_ACCESS_KEY_ID=]
        --aws_secret_access_key <aws_secret_access_key>    The AWS access secret key [env: AWS_SECRET_ACCESS_KEY=]
    -c, --count <count>
            Specifies the total count of transfers in the benchmark [default: 100]

    -e, --endpoint <endpoint>
            Sets the s3 endpoint [env: S3_ENDPOINT=]  [default: s3.amazonaws.com]

    -m, --mode <mode>
            The benchmarking mode, possibilities are :
            'r' or 'wr' == 'write then read''w' == 'write only'
            'mix##' == 'read and write simultaneously, the number indicates the percentage of reads vs writes' [default:
            wr]
    -p, --parallel <parallel>
            Sets the desired number of parallel in-flight requests [default: 8]

    -s, --size <size>
            Sets the individual payload size for reading and writing.
             Takes any form of size in bytes.. e.g. 1 byte | 5MB | 1Gb etc. [default: 1MB]
```
