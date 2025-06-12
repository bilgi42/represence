use std::error::Error;

pub async fn find_process(process_name:&str) -> Result<bool, Box<dyn Error + Send + Sync>> {

    let is_process_found = false;
    let mut result_text = String::new();
    let mut process_details = String::new();

    if is_process_found{
        println!("Process {} is found, details are here :\n{}", process_name, process_details);
    } else {
        println!("Process {} is NOT found", process_name)
    }

    Ok(is_process_found)
}