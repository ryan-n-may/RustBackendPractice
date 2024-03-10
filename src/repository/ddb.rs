use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::model::AttributeValue;
use aws_config::Config;
use crate::model::task::{Task, TaskState};
use log::error;
use std::str::FromStr;
use std::collections::HashMap;


/*
This struct is the interface between our API and dynamoDB
We made this to help ***US*** even though it looks fucking horrid at first glance.
 */
pub struct DDBRepository{
    client: Client, // This is a DynamoDB Client 
    table_name: String,
}

pub struct DDBError;

/*
Required Item Value differs from Item Value through the refusal for a required item value
to be NONE. (?)

Dont be consumed by Some() and its use here, it is seperate completely form Ok and Err.
Some() just allows the return of an Option which is an enum implementing a NULLABLE val. 
*/

// This is a function that takes a key and an item and returns a resule OK if the itemvalue is set.
// If the itemvalue is NONE, or an error occurs, the result returns Error.
fn required_item_value(key: &str, item: &HashMap<String, AttributeValue>) -> Result<String, DDBError> {
    match item_value(key, item) { // Returns an option to be matched. 
        Ok(Some(value)) => Ok(value),
        Ok(None) => Err(DDBError),
        Err(DDBError) => Err(DDBError)
    }
}

// This function is referenced by the above function.
fn item_value(key: &str, item: &HashMap<String, AttributeValue>) -> Result<Option<String>, DDBError> {
    match item.get(key) { // item.get(key) returns a value to be tested.
        Some(value) => match value.as_s() {  // If value is a string?
            Ok(val) => Ok(Some(val.clone())), 
            Err(_) => Err(DDBError)
        },
        None => Ok(None)
    }
}

// This method takes an item as input and outputs a Task
fn item_to_task(item: &HashMap<String, AttributeValue>) -> Result<Task, DDBError> {
    // assign state via the required item value "state" in the input item 
    // this converts the state string to the TaskState
    let state: TaskState = match TaskState::from_str(required_item_value("state", item)?.as_str()) {
        Ok(value) => value, 
        Err(_) => return Err(DDBError) // return ANY error
    };
    let result_file = item_value("result_file", item)?; // get item value from item

    // Return OK result (no Error) with this Task
    Ok(Task {
        user_uuid: required_item_value("pK", item)?, // get required item value from item
        task_uuid: required_item_value("sK", item)?, // get required item value from item
        task_type: required_item_value("task_type", item)?, // get required item value from item
        state,
        source_file: required_item_value("source_file", item)?,
        result_file,
    })
}

// Implementing the DDBRepository struct 
impl DDBRepository {
    pub fn init(table_name: String, config: Config) -> DDBRepository {
        let client = Client::new(&config);
        DDBRepository {
            table_name,
            client,
        }
    }

    // Putting a task into the DB
    pub async fn put_task(&self, task: Task) -> Result<(), DDBError> {
        // Constructing attribute values from strings 
        // Then constructing the request 
        let mut request = self.client.put_item() // DynamoDB crate methods
            .table_name(&self.table_name)
            .item("pK", AttributeValue::S(String::from(task.user_uuid)))
            .item("sK", AttributeValue::S(String::from(task.task_uuid)))
            .item("task_type", AttributeValue::S(String::from(task.task_type)))
            .item("state", AttributeValue::S(task.state.to_string()))
            .item("source_file", AttributeValue::S(task.source_file));

        // if task.result_file isnt NULL
        // create new request from task result file
        if let Some(result_file) = task.result_file {
            request = request.item("result_file", AttributeValue::S(String::from(result_file)));
        }
        // if request.send().await returns Ok, we are Ok, if ERR we throw a DDB Error
        match request.send().await {
            Ok(_) => Ok(()),
            Err(_) => Err(DDBError),
        }
    }

    // Fetching a task from the DynamoDB
    pub async fn get_task(&self, task_id: String) -> Option<Task> {

        let tokens : Vec<String> = task_id 
            .split("_")
            .map(|x| String::from(x)) // Closure creates a new itterator that conversts x to String objects.
            .collect(); // This collects the itterator into a Vec<String>

        let user_uuid = AttributeValue::S(tokens[0].clone()); // Call clone to copy string Heap data.
        let task_uuid = AttributeValue::S(tokens[1].clone());

        // Define res (part of DynamoDB)
        let res = self.client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("#pk = :user_id and #sK = :task_uuid")
            .expression_attribute_names("#pK", "pK")
            .expression_attribute_names("#sK", "sK")
            .expression_attribute_values(":user_id", user_uuid)
            .expression_attribute_values(":task_uuid", task_uuid)
            .send()
            .await;

        return match res {
            Ok(output) => { // if output is OK
                match output.items {
                    Some(items) => { // an Items are not NULL
                        let item = &items.first()?;
                        error!("{:?}", &item); // Print error message
                        match item_to_task(item) { // and task is OK
                            Ok(task) => Some(task), // return Option task
                            Err(_) => None // or Option NULL
                        }
                    },
                    None => {
                        None // or Option NULL
                    }
                }
            },
            Err(error) => {
                error!("{:?}", error); // Print error message
                None // Or option NULL
            }
        }

    }

}