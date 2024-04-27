#[macro_use]
extern crate serde;
use candid::{Decode, Encode, Result};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

// Define the structure for attendance
#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct Attendance {
    id: u64,
    employee_id: u64,
    time: u64,
}

// Define the structure for employee
#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct Employee {
    employee_id: u64,
    name: String,
    role: String,
}

// Implement serialization and deserialization for attendance
impl Storable for Attendance {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// Implement serialization and deserialization for employee
impl Storable for Employee {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// Implement bounded storable for attendance
impl BoundedStorable for Attendance {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Implement bounded storable for employee
impl BoundedStorable for Employee {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Define the possible errors
#[derive(candid::CandidType, Serialize, Deserialize)]
enum Error {
    NotFound { msg: String },
    AlreadyExists { msg: String },
}

// Thread-local storage for memory manager, ID Counter, attendance storage, and employee storage
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
        .expect("Cannot create a counter")
    );

    static ATTENDANCE_STORAGE: RefCell<StableBTreeMap<u64, Attendance, Memory>> = 
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(5)))
        ));

    static EMPLOYEE_STORAGE: RefCell<StableBTreeMap<u64, Employee, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(6)))
        ));
}

#[ic_cdk::query]
// Function to get attendance by ID
fn get_attendance(req_id: u64) -> Result<Attendance, Error> {
    match _get_attendance(&req_id) {
        Some(request) => Ok(request),
        None => Err(Error::NotFound {
            msg: format!("Attendance with ID {} not found", req_id),
        }),
        
    }
}

#[ic_cdk::update]
fn submit_attendance(employee_id: u64) -> Result<Attendance, Error> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = counter.borrow().get();
            counter.borrow_mut().set(current_value + 1);
    })
    .expect("Cannot increment counter");

    let attendance = Attendance {
        id,
        employee_id,
        time: time(),
    };

    ATTENDANCE_STORAGE.with(|service| service.borrow_mut().insert(id, attendance.clone()));

    Ok(attendance)
}

fn _get_attendance(req_id: &u64) -> Option<Attendance> {
    ATTENDANCE_STORAGE.with(|service| service.borrow().get(req_id))
}


#[ic_cdk::query]
fn get_employee(req_id: u64) -> Result<Employee, Error> {
    match _get_employee(&req_id) {
        Some(request) => Ok(request),
        None => Err(Error::NotFound {
            msg: format!("Employee with ID {} not found", req_id),
        }),
    }
}

#[ic_cdk::update]
fn add_employee(employee_id: u64, name: String, role: String) -> Result<Employee, Error> {
    let employee = Employee {
        employee_id,
        name,
        role,
    };

    EMPLOYEE_STORAGE.with(|service| service.borrow_mut().insert(employee_id, employee.clone()));

    Ok(employee)
}

#[ic_cdk::update]
fn update_employee(employee_id: u64, name: String, role: String) -> Result<Employee, Error> {
    EMPLOYEE_STORAGE.with(|service| service.borrow().get(&employee_id)) {
        Some(mut employee) => {
            employee.name = name;
            employee.role = role;
            service.borrow_mut().insert(employee_id, employee.clone());
            Ok(employee);
        },
        None => Err(Error::NotFound {
            msg: format!("Employee with ID {} not found", employee_id),
        }),
    }
    
}

fn _get_employee(req_id: &u64) -> Option<Employee> {
    EMPLOYEE_STORAGE.with(|service| service.borrow().get(req_id))
}

ic_cdk::export_candid!();