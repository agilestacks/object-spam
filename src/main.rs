extern crate futures;
extern crate rusoto_core;
extern crate rusoto_s3;
extern crate tokio;
extern crate time;
extern crate clap;
extern crate human_size;
extern crate quantiles;

mod duration;

#[macro_use]
extern crate lazy_static;

use std::str;
use std::iter;
use time::get_time;
use std::time::Duration;
use human_size::{SpecificSize, Byte};
use quantiles::histogram::{Bound, Histogram};
use clap::{Arg, ArgMatches, App};
use futures::{Future, Stream, stream};
use rusoto_core::{ Region, ProvideAwsCredentials};
use rusoto_core::credential::{ DefaultCredentialsProvider};
use rusoto_s3::{S3, S3Client, GetObjectRequest,
                PutObjectRequest, DeleteObjectRequest,
                CreateBucketRequest, DeleteBucketRequest };

use duration::DurationFuture;

lazy_static! {
    static ref matches : ArgMatches<'static> = App::new("object-spam")
                          .version("1.0")
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
                               .default_value("8")
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
}

fn main() {

    let region = 
        if *endpoint == "s3.amazonaws.com" {
            Region::UsEast1
        } else {
            // the name doesn't matter at this point
            Region::Custom { name: "us-east-1".to_owned(), endpoint: endpoint.to_owned() }
        };

    let client = S3Client::new(region.clone());
    let _credentials = DefaultCredentialsProvider::new().unwrap().credentials().wait().unwrap();
    let t = get_time();
    let bench_bucket = format!("object-spam-{}.{}", t.sec, t.nsec);

    println!("Creating test bucket {}", &bench_bucket);
    // Create bucket
    create_bucket(&client, &bench_bucket).wait().unwrap();

    println!("Working ...");
    // Write Payloads to bucket
    let writejobs = stream::iter_ok(1 .. *count).map(|i| {
        post_payload(&client, &bench_bucket, &format!("test_{}", i), &payload)
    })
    .buffered(*workers);

    let wtimings = writejobs.map(|(_, dur)| { 
        dur.as_secs() as f64 + (dur.subsec_micros() as f64 / 1_000_000_f64)
    }).collect().wait();

    // Read Payloads from bucket
    let readjobs = stream::iter_ok(1 .. *count).map(|i| {
        fetch_payload(&client, &bench_bucket, &format!("test_{}", i), *size)
    })
    .buffered(*workers);

    let rtimings = readjobs.map(|(_, dur)| { 
        dur.as_secs() as f64 + (dur.subsec_micros() as f64 / 1_000_000_f64)
    }).collect().wait();
  


    // Delete up payloads and bucket
    println!("Done! \nCleaning up ...");
    
    stream::iter_ok(1 .. *count).map(|i| {
        delete_payload(&client, &bench_bucket, &format!("test_{}", i))
    })
    .buffered(*workers).for_each(|_| Ok(())).wait().unwrap();

    client.delete_bucket(DeleteBucketRequest { bucket: bench_bucket.clone(), ..Default::default() }).wait().unwrap();
    
    println!("Stats");
    print_percentiles("Read", rtimings.unwrap());
    print_percentiles("Write", wtimings.unwrap());
}

fn create_bucket(client: &S3Client, bucket: &str) -> impl Future<Item=(), Error=String> {
    let create_bucket_req = CreateBucketRequest { bucket: bucket.to_owned(), ..Default::default() };
    client
        .create_bucket(create_bucket_req)
        .map_err(|e| e.to_string())
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
        .and_then(|_| Ok(()));

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
            resp.body.unwrap().concat2().map_err(|e| e.to_string())
        })
        .and_then(move |v| { 
            if v.len() == bufsz { Ok(()) } 
            else { Err(format!("Original size was {} and fetched payload size is {}", bufsz,  v.len())) }
        });
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

fn print_percentiles(category: &str, samples: Vec<f64>) {
    if samples.is_empty() {
        return;
    }
    let mut samples = samples.clone();
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let min = samples.first().unwrap();
    let max = samples.last().unwrap();
    let fifty = max * 0.5;
    let seventy = max * 0.7; 
    let ninety = max * 0.9;
    let ninety5 = max * 0.95;
    let ninety9 = max * 0.99;
    let mut hist = Histogram::new(vec![fifty, seventy, ninety, ninety5, ninety9]).unwrap();
    for s in samples.iter() {
        hist.insert(*s);
    }

    println!("Timing results for {}", category);
    println!("Mean = {}", fifty);
    println!("Max = {}", max);
    println!("Min = {}", min);
    println!("50th Percenile value = {} | count = {}", fifty, hist.total_above(Bound::Finite(fifty)));
    println!("70th Percenile value = {} | count = {}", seventy, hist.total_above(Bound::Finite(seventy)));
    println!("90th Percenile value = {} | count = {}", ninety, hist.total_above(Bound::Finite(ninety)));
    println!("95th Percenile value = {} | count = {}", ninety5, hist.total_above(Bound::Finite(ninety5)));
    println!("99th Percenile value = {} | count = {}", ninety9, hist.total_above(Bound::Finite(ninety9)));
}
