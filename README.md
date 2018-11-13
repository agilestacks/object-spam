

## Object Spam

A simple benchmarking tool with tunable parallelism, loads, and durations.  
Reports basic stats and histograms of read and write execution



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
