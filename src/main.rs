extern crate futures;
extern crate rusoto_core;
extern crate rusoto_s3;
extern crate tokio;
extern crate time;
extern crate clap;
extern crate human_size;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;

mod duration;
mod stats;

#[macro_use]
extern crate lazy_static;

use std::str;
use std::iter;
use std::sync::Arc;
use time::get_time;
use std::time::{ Duration, Instant };
use human_size::{SpecificSize, Byte };
use clap::{Arg, ArgMatches, App};
use tokio::runtime::Runtime;
use futures::{Future, Stream, stream, future};
use rusoto_core::{ Region, ProvideAwsCredentials};
use rusoto_core::credential::{ DefaultCredentialsProvider};
use rusoto_s3::{S3, S3Client, GetObjectRequest,
                PutObjectRequest, DeleteObjectRequest,
                CreateBucketRequest, DeleteBucketRequest };

use duration::DurationFuture;
use stats::Stats;

lazy_static! {
    static ref matches : ArgMatches<'static> = App::new("object-spam")
                          .version("0.1")
                          .author("Rick Richardson <rick@agilestacks.com>")
                          .about("Object Storage benchmarking tool")
                          .arg(Arg::with_name("size")
                               .short("s")
                               .long("size")
                               .default_value("1MB")
                               .value_name("size")
                               .help("Sets the individual payload size for reading and writing.\n Takes any form of size in bytes.. e.g. 1 byte | 5MB | 1Gb etc.")
                               .takes_value(true))
                          .arg(Arg::with_name("parallel")
                               .help("Sets the desired number of parallel in-flight requests")
                               .short("p")
                               .long("parallel")
                               .default_value("3")
                               .takes_value(true))
                          .arg(Arg::with_name("count")
                               .short("c")
                               .long("count")
                               .default_value("100")
                               .help("Specifies the total count of transfers in the benchmark")
                               .takes_value(true))
                          .arg(Arg::with_name("endpoint")
                               .short("e")
                               .long("endpoint")
                               .default_value("s3.amazonaws.com")
                               .env("S3_ENDPOINT")
                               .takes_value(true)
                               .help("Sets the s3 endpoint"))
                          .arg(Arg::with_name("mode")
                               .short("m")
                               .long("mode")
                               .default_value("wr")
                               .help("The benchmarking mode, possibilities are : \n'r' or 'wr' == 'write then read'\'w' == 'write only'\n'mix##' == 'read and write simultaneously, the number indicates the percentage of reads vs writes'")
                               .takes_value(true))
                          .arg(Arg::with_name("json")
                               .long("json")
                               .short("j")
                               .takes_value(false)
                               .help("Produce output in json instead of ascii prettyness"))
                          .arg(Arg::with_name("aws_access_key_id")
                               .long("aws_access_key_id")
                               .help("The AWS access key id")
                               .env("AWS_ACCESS_KEY_ID")
                               .takes_value(true))
                          .arg(Arg::with_name("aws_secret_access_key")
                               .long("aws_secret_access_key")
                               .help("The AWS access secret key")
                               .env("AWS_SECRET_ACCESS_KEY")
                               .takes_value(true))
                          .get_matches();

    static ref sz : SpecificSize<Byte> = matches.value_of("size").unwrap().parse().unwrap();
    static ref size : usize = sz.value() as usize;
    static ref payload : Vec<u8> = iter::repeat('z' as u8).take(*size).collect();
    static ref workers : usize = matches.value_of("parallel").unwrap().parse().unwrap();
    static ref endpoint : &'static str = matches.value_of("endpoint").unwrap();
    static ref count : usize = matches.value_of("count").unwrap().parse().unwrap(); 
    static ref jsonout : bool = matches.is_present("json");
    static ref bench_bucket : String = { 
        let t = get_time();
        format!("object-spam-{}.{}", t.sec, t.nsec)
    };
}

fn main() {

    let region = 
        if *endpoint == "s3.amazonaws.com" {
            Region::UsEast1
        } else {
            // the name doesn't matter at this point
            Region::Custom { name: "us-east-1".to_owned(), endpoint: endpoint.to_owned() }
        };
    let client = Arc::new(S3Client::new(region.clone()));
    let mut core = Runtime::new().unwrap();

    let _credentials = DefaultCredentialsProvider::new().unwrap().credentials().wait().unwrap();

    if !*jsonout {
        println!("Creating test bucket {}", &*bench_bucket);
    }
    // Create bucket
    core.block_on(create_bucket(&*client.clone(), &*bench_bucket)).unwrap();

    if !*jsonout {
        println!("Working ...");
    }
    let start_write = Instant::now();
    // Write Payloads to bucket
    let c1 = client.clone();
    let writejobs = stream::iter_ok(1 .. *count).map(move |i| {
            post_payload(&*c1, &bench_bucket, &format!("test_{}", i), &payload)
        })
        .buffered(*workers)
        .map(|(_, dur)| { 
            dur.as_secs() as f64 + (dur.subsec_micros() as f64 / 1_000_000_f64)
        }).collect();

    let wtimings = core.block_on(writejobs).unwrap();

    // Read Payloads from bucket
    let c2 = client.clone();
    let start_read = Instant::now();
    let readjobs = stream::iter_ok(1 .. *count).map(move |i| {
            fetch_payload(&*c2, &bench_bucket, &format!("test_{}", i), *size)
        })
        .buffered(*workers)
        .map(|(_, dur)| { 
            dur.as_secs() as f64 + (dur.subsec_micros() as f64 / 1_000_000_f64)
        }).collect();
  
    let rtimings = core.block_on(readjobs).unwrap();

    // Delete up payloads and bucket
    if !*jsonout {
        println!("Done! \nCleaning up ...");
    }
  
    let c3 = client.clone();
    let start_clean = Instant::now();
    let delstuff = stream::iter_ok(1 .. *count).map(move |i| {
        delete_payload(&*c3, &bench_bucket, &format!("test_{}", i))
    })
    .buffered(*workers).for_each(|_| Ok(()))
    .and_then(move |_| 
        client.delete_bucket(DeleteBucketRequest { bucket: bench_bucket.clone(), ..Default::default() })
        .map_err(|e| e.to_string())
        .and_then(|_| Ok(())));

    core.block_on(delstuff).unwrap();
  
    let end_time = Instant::now();
    core.shutdown_on_idle().wait().unwrap();
    let wtime = start_read.duration_since(start_write).as_secs();
    let rtime = start_clean.duration_since(start_read).as_secs();
    let total = end_time.duration_since(start_write).as_secs();

    if *jsonout {
        println!("{{ \"read\": {}, \n \"write\": {},", Stats::new("read", rtimings, *size).to_json(), Stats::new("write", wtimings, *size).to_json());
        println!("\"read_time\": {}, \"write_time\": {}, \"total_time\": {} }}", rtime, wtime, total);
    } else {
        println!("-= Stats =-");
        println!("{}", Stats::new("read", rtimings, *size));
        println!("{}", Stats::new("write", wtimings, *size));
        println!("Write time = {} s", wtime);
        println!("Read time = {} s", rtime);
        println!("Total time = {} s", total);
    }
}

fn create_bucket(client: &S3Client, bucket: &str) -> impl Future<Item=(), Error=()> {
    let create_bucket_req = CreateBucketRequest { bucket: bucket.to_owned(), ..Default::default() };
    client
        .create_bucket(create_bucket_req)
        .map_err(|e| println!("Failed to create bucket: {}", e.to_string()))
        .and_then(|_| Ok(()))
}

fn post_payload(client: &S3Client, bucket: &str, key: &str, buffer: &Vec<u8>) -> impl Future<Item=((), Duration), Error=String> {
    let req = PutObjectRequest {
        bucket: bucket.to_owned(),
        key: key.to_owned(),
        body: Some(buffer.clone().into()),
        ..Default::default()
    };
    let result = client
        .put_object(req)
        .map_err(|e| e.to_string())
        .and_then(|_| Ok(()))
        .or_else(|e| Ok(println!("Error: {}", e)));

    DurationFuture::new(result)
}

fn fetch_payload(client: &S3Client, bucket: &str, key: &str, bufsz: usize) -> impl Future<Item=((), Duration), Error=String> {
    let get_req = GetObjectRequest {
        bucket: bucket.to_owned(),
        key: key.to_owned(),
        ..Default::default()
    };

    let result = client
        .get_object(get_req)
        .map_err(|e| e.to_string())
        .and_then(|resp| {
            resp.body.unwrap().map_err(|e| format!("{}", e)).fold(0, |acc : usize, x : Vec<u8>| future::ok::<usize, String>(acc + x.len()))
        })
        .and_then(move |len| { 
            if len == bufsz { Ok(()) } 
            else { Err(format!("Original size was {} and fetched payload size is {}", bufsz,  len)) }
        })
        .or_else(|e| Ok(println!("Error: {}", e)));
    DurationFuture::new(result)
}

fn delete_payload(client: &S3Client, bucket: &str, key: &String) -> impl Future<Item=(), Error=String> {
    let del_req = DeleteObjectRequest {
        bucket: bucket.to_owned(),
        key: key.to_owned(),
        ..Default::default()
    };
    client.delete_object(del_req) 
        .map_err(|e| e.to_string())
        .and_then(|_| Ok(()))
}

