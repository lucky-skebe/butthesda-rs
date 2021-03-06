use std::{collections::HashSet, ffi::c_void, fmt::Display, mem::size_of, time::Duration};

use futures::StreamExt;
use sysinfo::{ProcessExt, System, SystemExt};
use tracing::debug;
use winapi::um::{
    memoryapi::{
        ReadProcessMemory, VirtualAllocEx, VirtualProtectEx, VirtualQueryEx, WriteProcessMemory,
    },
    processthreadsapi::OpenProcess,
    tlhelp32::{CreateToolhelp32Snapshot, Module32First, MODULEENTRY32, TH32CS_SNAPMODULE},
    winnt::{
        MEMORY_BASIC_INFORMATION, MEM_COMMIT, MEM_FREE, MEM_RESERVE, PAGE_EXECUTE_READWRITE,
        PAGE_READWRITE, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
    },
};

use crate::GameState;

#[derive(Debug, Clone, Copy)]
pub struct ProcessInfo {
    process_name: &'static str,
    pattern: &'static [u8],
    program: &'static [u8],
    timer_offsets: &'static [isize],
    hook_len: usize,
    inject_offset: isize,
    data_offset: isize,
    replace_base_address_offset: usize,
    replace_data_offset: usize,
    replace_return_offset: usize,
    is_64_bit: bool,
}

// pub static SKYRIM: ProcessInfo = ProcessInfo {
//     process_name: "TESV.exe",
//     pattern: &[
//         0x8B, 0x44, 0x24, 0x04, 0x81, 0xEC, 0x08, 0x01, 0x00, 0x00, 0x53, 0x56, 0x57, 0x8B, 0x38,
//         0x8B, 0xC7, 0x32, 0xDB, 0x8D, 0x50, 0x01, 0x8A, 0x08, 0x40, 0x84, 0xC9, 0x75, 0xF9, 0x2B,
//         0xC2,
//     ],
//     program: &[
//         0x8B, 0xF0, 0x50, 0xB8, 0xA6, 0x00, 0xF1, 0x01, 0x83, 0xC0, 0x50, 0x89, 0x08, 0x8B, 0xC8,
//         0x83, 0xC1, 0x08, 0x58, 0x89, 0x01, 0x83, 0xC1, 0x08, 0x89, 0x11, 0x83, 0xC1, 0x08, 0x58,
//         0x89, 0x01, 0x83, 0xC1, 0x08, 0x58, 0x89, 0x01, 0x58, 0x8B, 0xD0, 0x50, 0x8B, 0x01, 0x50,
//         0x83, 0xE9, 0x08, 0x8B, 0x01, 0x50, 0x83, 0xE9, 0x08, 0xFF, 0x31, 0x83, 0xE9, 0x08, 0xFF,
//         0x31, 0x83, 0xE9, 0x08, 0xFF, 0x31, 0xB8, 0x00, 0x00, 0x40, 0x00, 0x05, 0x3C, 0x06, 0xF1,
//         0x00, 0x8B, 0x00, 0x83, 0xC0, 0x74, 0x8B, 0x00, 0x83, 0xC0, 0x04, 0x8B, 0x00, 0x05, 0x00,
//         0x01, 0x00, 0x00, 0x8B, 0x00, 0x83, 0xC0, 0x10, 0x8B, 0x00, 0x83, 0xC0, 0x38, 0x39, 0xC2,
//         0x0F, 0x85, 0x2C, 0x00, 0x00, 0x00, 0xB8, 0xA6, 0x00, 0xF1, 0x01, 0x05, 0xF0, 0x00, 0x00,
//         0x00, 0x83, 0xC0, 0x10, 0x39, 0x38, 0x0F, 0x84, 0x0D, 0x00, 0x00, 0x00, 0x83, 0x38, 0x00,
//         0x0F, 0x84, 0x02, 0x00, 0x00, 0x00, 0xEB, 0xEA, 0x89, 0x38, 0x83, 0xC0, 0x08, 0x8B, 0x18,
//         0x83, 0xC3, 0x01, 0x89, 0x18, 0x5A, 0x58, 0x58, 0x83, 0xFE, 0x04, 0xE9, 0x00, 0x00, 0x00,
//         0x00,
//     ],
//     timer_offsets: &[0xF10588, 0x88, 0x4, 0x100, 0x10, 0x98, 0x58, 0x0, 0x44],
//     hook_len: 5,
//     inject_offset: 0x1f,
//     data_offset: 0x100,
//     replace_base_address_offset: 0x04,
//     replace_data_offset: 0x70,
//     replace_return_offset: 0xA2,
// };

pub static SKYRIM_SE: ProcessInfo = ProcessInfo {
    process_name: "SkyrimSE.exe",
    pattern: &[
        0x48, 0x8B, 0xC4, 0x57, 0x48, 0x81, 0xEC, 0x40, 0x01, 0x00, 0x00, 0x48, 0xC7, 0x44, 0x24,
        0x20, 0xFE, 0xFF, 0xFF, 0xFF, 0x48, 0x89, 0x58, 0x10, 0x48, 0x89, 0x70, 0x18,
    ],
    program: &[
        0x48, 0x8B, 0xF2, //mov rsi,rdx                                   <-newmem
        0x48, 0x8B, 0x39, //mov rdi,[rcx]
        0x51, //
        0x50, //
        0x48, 0xB8, 0x00, 0x00, 0xEC, 0x4E, 0xF7, 0x7F, 0x00,
        0x00, //mov rax,SkyrimSE.exe      SkyrimSE.exe
        0x48, 0x05, 0xC8, 0x47, 0xEC, 0x01, //                          1st offset
        0x48, 0x8B, 0x00, //mov rax,[rax]
        0x48, 0x05, 0xD0, 0x00, 0x00, 0x00, //                          2th offset
        0x48, 0x8B, 0x00, //mov rax,[rax]
        0x48, 0x83, 0xC0, 0x08, //                          3th offset
        0x48, 0x8B, 0x00, //mov rax,[rax]
        0x48, 0x05, 0xA8, 0x01, 0x00, 0x00, //                          4th offset
        0x48, 0x8B, 0x00, //mov rax,[rax]
        0x48, 0x05, 0x90, 0x00, 0x00, 0x00, //                          5tffset
        0x48, 0x8B, 0x00, //mov rax,[rax]
        0x48, 0x83, 0xC0, 0x68, //                          6th offset
        0x4C, 0x39, 0xF0, 0x0F, 0x85, 0x39, 0x00, 0x00, 0x00, //jne finishUp
        0x48, 0xB8, 0x8A, 0x00, 0xEB, 0xC4, 0xF7, 0x7F, 0x00,
        0x00, //mov rax,randomData        randomData
        0x48, 0x05, 0xF0, 0x00, 0x00, 0x00, 0x48, 0x83, 0xC0,
        0x10, //                                              <-increaseArray
        0x48, 0x39, 0x38, 0x0F, 0x84, 0x0E, 0x00, 0x00, 0x00, //je countUoArrayItem
        0x83, 0x38, 0x00, 0x0F, 0x84, 0x02, 0x00, 0x00, 0x00, //je createNewArrayItem
        0xEB, 0xE8, //jmp increaseArray                             <-createNewArrayItem
        0x48, 0x89, 0x38, //                                              <-countUpArrayItem
        0x48, 0x83, 0xC0, 0x08, //
        0x48, 0x8B, 0x08, //
        0x48, 0x83, 0xC1, 0x01, //
        0x48, 0x89, 0x08, //
        0x58, //pop rax                                       <-finishUp
        0x59, //pop rcx
        0xE9, 0x28, 0x1C, 0x1B,
        0x00, //jmp INJECT                INJECT
              //      <-randomData
    ],
    timer_offsets: &[0x01EC47C8, 0xD0, 0x8, 0x1B0, 0x20, 0x118, 0x98, 0x0, 0x44],
    hook_len: 6,
    inject_offset: 0x1C,
    data_offset: 0x100,
    replace_base_address_offset: 0x0A,
    replace_data_offset: 0x4C,
    replace_return_offset: 0x86,
    is_64_bit: true,
};

pub static SKYRIM_VR: ProcessInfo = ProcessInfo {
    process_name: "SkyrimVR.exe",
    pattern: &[
        0x48, 0x8B, 0xC4, 0x57, 0x48, 0x81, 0xEC, 0x40, 0x01, 0x00, 0x00, 0x48, 0xC7, 0x44, 0x24,
        0x20, 0xFE, 0xFF, 0xFF, 0xFF, 0x48, 0x89, 0x58, 0x10, 0x48, 0x89, 0x70, 0x18,
    ],
    program: &[
        0x48, 0x8B, 0xF2, //mov rsi,rdx                                   <-newmem
        0x48, 0x8B, 0x39, //mov rdi,[rcx]
        0x51, 0x50, 0x48, 0xB8, 0x00, 0x00, 0xEC, 0x4E, 0xF7, 0x7F, 0x00,
        0x00, //mov rax,SkyrimSE.exe      SkyrimSE.exe
        0x48, 0x05, 0x68, 0x96, 0xF8, 0x01, //                          1st offset
        0x48, 0x8B, 0x00, //mov rax,[rax]
        0x48, 0x05, 0xD0, 0x00, 0x00, 0x00, //                          2th offset
        0x48, 0x8B, 0x00, //mov rax,[rax]
        0x48, 0x83, 0xC0, 0x08, //                          3th offset
        0x48, 0x8B, 0x00, //mov rax,[rax]
        0x48, 0x05, 0xA8, 0x01, 0x00, 0x00, //                          4th offset
        0x48, 0x8B, 0x00, //mov rax,[rax]
        0x48, 0x05, 0x90, 0x00, 0x00, 0x00, //                          5tffset
        0x48, 0x8B, 0x00, //mov rax,[rax]
        0x48, 0x83, 0xC0, 0x68, //                          6tffset
        0x4C, 0x39, 0xF0, 0x0F, 0x85, 0x39, 0x00, 0x00, 0x00, //jne finishUp
        0x48, 0xB8, 0x8A, 0x00, 0xEB, 0xC4, 0xF7, 0x7F, 0x00,
        0x00, //mov rax,randomData        randomData
        0x48, 0x05, 0xF0, 0x00, 0x00, 0x00, 0x48, 0x83, 0xC0,
        0x10, //                                              <-increaseArray
        0x48, 0x39, 0x38, 0x0F, 0x84, 0x0E, 0x00, 0x00, 0x00, //je countUoArrayItem
        0x83, 0x38, 0x00, 0x0F, 0x84, 0x02, 0x00, 0x00, 0x00, //je createNewArrayItem
        0xEB, 0xE8, //jmp increaseArray                             <-createNewArrayItem
        0x48, 0x89, 0x38, //                                              <-countUpArrayItem
        0x48, 0x83, 0xC0, 0x08, 0x48, 0x8B, 0x08, 0x48, 0x83, 0xC1, 0x01, 0x48, 0x89, 0x08,
        0x58, //pop rax                                       <-finishUp
        0x59, //pop rcx
        0xE9, 0x28, 0x1C, 0x1B,
        0x00, //jmp INJECT                INJECT
              //                                              <-randomData
    ],
    timer_offsets: &[
        0x01F89668, 0xD0, 0x8, 0x1A8, 0x48, 0x0, 0x118, 0x98, 0x0, 0x44,
    ],
    hook_len: 6,
    inject_offset: 0x1C,
    data_offset: 0x100,
    replace_base_address_offset: 0x0A,
    replace_data_offset: 0x4C,
    replace_return_offset: 0x86,
    is_64_bit: true,
};

// pub static FALLOUT_4: ProcessInfo = ProcessInfo {
//     process_name: "Fallout 4.exe",
//     pattern: &[
//         0x48, 0x8B, 0xC4, 0x57, 0x48, 0x81, 0xEC, 0x40, 0x01, 0x00, 0x00, 0x48, 0xC7, 0x44, 0x24,
//         0x20, 0xFE, 0xFF, 0xFF, 0xFF, 0x48, 0x89, 0x58, 0x10, 0x48, 0x89, 0x70, 0x18,
//     ],
// };

#[derive(Debug)]
pub struct ProcessMemory {
    handle: *mut c_void,
    base_address: *mut u8,
    is_64_bit: bool,
}

impl ProcessMemory {
    fn read_raw_bytes(
        &self,
        address: *const u8,
        target: *mut u8,
        length: usize,
    ) -> (i32, Result<(), Error>) {
        let mut bytes_read = 0;

        let ok = unsafe {
            ReadProcessMemory(
                self.handle,
                address as *const _,
                target as *mut _,
                length,
                &mut bytes_read as *mut _ as *mut usize,
            )
        };

        (
            bytes_read,
            if ok == 0 {
                Err(unsafe { Error(winapi::um::errhandlingapi::GetLastError()) })
            } else {
                Ok(())
            },
        )
    }

    fn write_raw_bytes(
        &mut self,
        address: *mut u8,
        target: *mut u8,
        length: usize,
    ) -> (i32, Result<(), Error>) {
        let mut bytes_writter = 0;

        let ok = unsafe {
            WriteProcessMemory(
                self.handle,
                address as *mut _,
                target as *mut _,
                length,
                &mut bytes_writter as *mut _ as *mut usize,
            )
        };

        (
            bytes_writter,
            if ok == 0 {
                Err(unsafe { Error(winapi::um::errhandlingapi::GetLastError()) })
            } else {
                Ok(())
            },
        )
    }

    pub fn read<T>(&self, address: *mut u8) -> Result<T, Error>
    where
        T: Default,
    {
        let mut data = T::default();
        let len = std::mem::size_of::<T>();

        let (bytes_read, res) = self.read_raw_bytes(address, &mut data as *mut _ as *mut u8, len);

        let _ = res?;

        if bytes_read as usize != len {
            //err
            todo!()
        } else {
            Ok(data)
        }
    }

    fn read_slice(&self, address: *mut u8, slice: &mut [u8]) -> Result<i32, Error> {
        let (bytes_read, result) =
            self.read_raw_bytes(address, slice as *mut _ as *mut u8, slice.len());

        match result {
            Ok(_) => Ok(bytes_read),
            Err(Error(299)) => Ok(bytes_read),
            Err(e) => Err(e),
        }
    }

    fn write_slice(&mut self, address: *mut u8, slice: &mut [u8]) -> Result<i32, Error> {
        let (bytes_read, result) =
            self.write_raw_bytes(address, slice as *mut _ as *mut u8, slice.len());

        match result {
            Ok(_) => Ok(bytes_read),
            Err(Error(299)) => Ok(bytes_read),
            Err(e) => Err(e),
        }
    }

    fn allocate_memory(&mut self, mut address: *mut u8, size: usize) -> Result<*mut c_void, Error> {
        let mut mbi = MEMORY_BASIC_INFORMATION {
            BaseAddress: std::ptr::null_mut(),
            AllocationBase: std::ptr::null_mut(),
            AllocationProtect: 0,
            RegionSize: 0,
            State: 0,
            Protect: 0,
            Type: 0,
        };

        while unsafe {
            VirtualQueryEx(
                self.handle,
                address as *const _,
                &mut mbi as *mut _,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        } != 0
        {
            if mbi.State == MEM_FREE {
                let addr = unsafe {
                    VirtualAllocEx(
                        self.handle,
                        mbi.BaseAddress,
                        size,
                        MEM_COMMIT | MEM_RESERVE,
                        PAGE_EXECUTE_READWRITE,
                    )
                };

                if !addr.is_null() {
                    return Ok(addr);
                }
            }
            address = unsafe { address.offset(-0x10000) };
        }

        Err(Error(8))
    }

    pub fn read_ptr(&self, address: *mut u8) -> Result<*mut u8, Error> {
        Ok(if self.is_64_bit {
            self.read::<u64>(address)? as *mut u8
        } else {
            self.read::<u32>(address)? as *mut u8
        })
    }

    pub fn read_ptr_chain(&self, offsets: &[isize]) -> Result<*mut u8, Error> {
        let mut addr = self.base_address;
        for offset in &offsets[..offsets.len() - 1] {
            addr = self.read_ptr(addr.wrapping_offset(*offset))?;
        }

        Ok(addr.wrapping_offset(offsets[offsets.len() - 1]))
    }

    pub fn read_str(&self, address: *mut u8, len: usize) -> Result<String, Error> {
        let mut buff = Vec::new();
        buff.resize(len, 0);
        let bytes_read = self.read_slice(address, &mut buff)? as usize;

        let bytes: Vec<_> = buff[..bytes_read]
            .into_iter()
            .take_while(|b| **b != 0)
            .cloned()
            .collect();

        Ok(String::from_utf8(bytes).unwrap())
    }

    pub fn virtual_protect_ex(
        &self,
        address: *mut u8,
        len: usize,
        protect: winapi::shared::minwindef::DWORD,
    ) -> Result<u32, Error> {
        let mut old_protect: u32 = 0;

        let ok = unsafe {
            VirtualProtectEx(
                self.handle,
                address as *mut _,
                len,
                protect,
                &mut old_protect as *mut _,
            )
        };

        if ok == 0 {
            Err(unsafe { Error(winapi::um::errhandlingapi::GetLastError()) })
        } else {
            Ok(old_protect)
        }
    }
}

#[derive(Debug)]
pub struct Error(u32);

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ptr_buffer = std::ptr::null_mut();

        unsafe {
            let len = winapi::um::winbase::FormatMessageA(
                winapi::um::winbase::FORMAT_MESSAGE_ALLOCATE_BUFFER
                    | winapi::um::winbase::FORMAT_MESSAGE_FROM_SYSTEM
                    | winapi::um::winbase::FORMAT_MESSAGE_IGNORE_INSERTS,
                std::ptr::null(),
                self.0,
                0,
                ptr_buffer,
                0,
                std::ptr::null_mut(),
            );

            if len != 0 {
                let cstr = std::ffi::CStr::from_ptr(ptr_buffer);
                match cstr.to_str() {
                    Ok(str) => {
                        write!(f, "Native Error ({} {})", self.0, str)
                    }
                    Err(_) => {
                        write!(f, "Native Error ({})", self.0)
                    }
                }
            } else {
                write!(f, "Native Error ({})", self.0)
            }
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug)]
pub struct Process {
    memory: ProcessMemory,
    pid: u32,
    info: ProcessInfo,
}

unsafe impl Send for Process {}

impl Process {
    #[tracing::instrument]
    pub fn open(info: ProcessInfo) -> Result<Option<Self>, Error> {
        let mut sys = System::new();
        sys.refresh_processes();
        let process = sys.process_by_name(info.process_name);
        if let Some(process) = process.get(0) {
            let pid = process.pid() as u32;

            unsafe {
                let handle = OpenProcess(
                    PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION,
                    0,
                    pid,
                );

                if handle.is_null() {
                    return Err(Error(winapi::um::errhandlingapi::GetLastError()));
                }

                let mut entry = MODULEENTRY32 {
                    dwSize: size_of::<MODULEENTRY32>() as u32,
                    GlblcntUsage: Default::default(),
                    ProccntUsage: Default::default(),
                    hModule: std::ptr::null_mut(),
                    modBaseAddr: std::ptr::null_mut(),
                    modBaseSize: Default::default(),
                    szExePath: [0; 260],
                    szModule: [0; 256],
                    th32ModuleID: Default::default(),
                    th32ProcessID: Default::default(),
                };

                let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE, pid);
                if snapshot.is_null() {
                    return Err(Error(winapi::um::errhandlingapi::GetLastError()));
                }

                let main_module = Module32First(snapshot, &mut entry as *mut MODULEENTRY32);
                if main_module == 0 {
                    return Err(Error(winapi::um::errhandlingapi::GetLastError()));
                }

                let memory = ProcessMemory {
                    base_address: entry.modBaseAddr,
                    handle,
                    is_64_bit: info.is_64_bit,
                };

                Ok(Some(Self { pid, info, memory }))
            }
        } else {
            Ok(None)
        }
    }

    pub fn hook(&mut self, hook: *mut u8, func: *mut u8, len: usize) -> Result<(), Error> {
        let old_protect = self.memory.virtual_protect_ex(hook, len, PAGE_READWRITE)?;

        let mut empty_bytes = Vec::with_capacity(len);
        empty_bytes.resize(len, 0x90u8);

        self.memory.write_slice(hook, &mut empty_bytes)?;

        let byte_jump_delta = (func as isize - hook as isize - 5).to_le_bytes();

        let mut managed_array = [
            0xE9,
            byte_jump_delta[0],
            byte_jump_delta[1],
            byte_jump_delta[2],
            byte_jump_delta[3],
        ];

        self.memory.write_slice(hook, &mut managed_array)?;

        self.memory.virtual_protect_ex(hook, len, old_protect)?;

        Ok(())
    }

    #[tracing::instrument]
    fn find_pattern(&mut self) -> Result<Option<*mut u8>, Error> {
        let mut buffer: [u8; 1024] = [0; 1024];
        let mut next: [u8; 1024] = [0; 1024];

        let mut offset = 0;

        let bytes_read;

        unsafe {
            bytes_read = self
                .memory
                .read_slice(self.memory.base_address.offset(offset), &mut buffer)?
                as usize;
        }

        while bytes_read == 1024 {
            let bytes_read = self.memory.read_slice(
                unsafe { self.memory.base_address.offset(offset + 1024) },
                &mut next,
            )? as usize;

            for i in 0..1024 {
                let mut all_matched = true;
                for (matched_index, p) in self.info.pattern.iter().enumerate() {
                    if *p == 0 {
                        continue;
                    }

                    let index = i + matched_index;

                    if index >= 1024 {
                        let index = index - 1024;

                        if index >= bytes_read {
                            return Ok(None);
                        }

                        if next[index] != *p {
                            all_matched = false;
                            break;
                        }
                    } else {
                        if buffer[index] != *p {
                            all_matched = false;
                            break;
                        }
                    }
                }

                if all_matched {
                    return Ok(Some(unsafe { self.memory.base_address.offset(offset) }));
                }

                offset += 1;
            }

            buffer = next;
        }

        Ok(None)
    }

    pub fn inject(mut self) -> Result<Option<InjectedProcess>, Error> {
        let ptr_data = if let Ok(Some(ptr)) = self.find_pattern() {
            let ptr = unsafe { ptr.offset(self.info.inject_offset) };

            let program = self.info.program;
            let program_len = program.len();

            let check = self.memory.read::<u8>(ptr)?;

            let ptr_data = if check == 0xE9 {
                let ptr_plus1 = unsafe { ptr.offset(0x01) };

                let offset = self.memory.read::<i32>(ptr_plus1)? as isize;

                unsafe { ptr.offset(offset + 5 + program_len as isize + self.info.data_offset) }
            } else {
                let ptr_function = self.memory.allocate_memory(ptr, 10000)? as *mut u8;
                let ptr_data = unsafe { ptr_function.offset(program_len as isize) };

                let mut bytes = [0u8; 10000];

                bytes[0..program_len].copy_from_slice(&program);

                for (offset, b) in (self.memory.base_address as usize)
                    .to_le_bytes()
                    .iter()
                    .enumerate()
                {
                    bytes[self.info.replace_base_address_offset + offset] = *b;
                }

                for (offset, b) in (ptr_data as usize).to_le_bytes().iter().enumerate() {
                    bytes[self.info.replace_data_offset + offset] = *b;
                }

                let address_bytes =
                    (ptr as usize + self.info.hook_len - ptr_function as usize - 0x8A)
                        .to_le_bytes();
                dbg!(ptr, ptr, ptr_function, address_bytes);
                for (offset, b) in address_bytes.iter().enumerate() {
                    bytes[self.info.replace_return_offset + offset] = *b;
                }

                self.memory.write_slice(ptr_function, &mut bytes)?;

                self.hook(ptr, ptr_function, self.info.hook_len)?;

                ptr_data.wrapping_offset(self.info.data_offset)
            };

            Some(ptr_data)
        } else {
            None
        };

        Ok(ptr_data.map(|ptr_data| InjectedProcess {
            memory: self.memory,
            _pid: self.pid,
            ptr_data,
            timer_offsets: self.info.timer_offsets,
        }))
    }
}

impl Drop for ProcessMemory {
    fn drop(&mut self) {
        unsafe {
            winapi::um::handleapi::CloseHandle(self.handle);
        }
    }
}

pub struct InjectedProcess {
    memory: ProcessMemory,
    _pid: u32,
    ptr_data: *mut u8,
    timer_offsets: &'static [isize],
}

unsafe impl Send for InjectedProcess {}
unsafe impl Sync for InjectedProcess {}

impl InjectedProcess {
    fn timer_addr(&self) -> Result<*mut u8, Error> {
        let ptr_timer = self.memory.read_ptr_chain(self.timer_offsets)?;

        Ok(ptr_timer)
    }

    pub fn timer(&self) -> Result<f32, Error> {
        let addr = self.timer_addr()?;

        self.memory.read::<f32>(addr)
    }

    pub fn animation_list(&self) -> Result<HashSet<(String, u32)>, Error> {
        let mut ptr = self.ptr_data;
        let mut ret = HashSet::new();

        loop {
            let name_address = self.memory.read_ptr(ptr)?;
            if !name_address.is_null() {
                let name = self.memory.read_str(name_address, 30)?;
                let amount = self.memory.read::<u32>(ptr.wrapping_offset(0x08))?;
                ret.insert((name, amount));
                ptr = ptr.wrapping_offset(0x10);
            } else {
                break;
            }
        }

        Ok(ret)
    }
}

fn compate_animations(old: &HashSet<(String, u32)>, new: &HashSet<(String, u32)>) -> Vec<String> {
    new.difference(old).map(|(name, _)| name.clone()).collect()
}

#[derive(Debug, Clone)]
pub enum Message {
    TimerReset,
    AnimationsChanged(Vec<String>),
    GameStateChanged(GameState),
}

pub async fn run(
    message_bus: tokio::sync::broadcast::Sender<crate::Message>,
) -> anyhow::Result<()> {
    let mut in_box = Box::pin(
        tokio_stream::wrappers::BroadcastStream::new(message_bus.subscribe()).filter_map(
            |m| async {
                match m {
                    Ok(crate::Message::ConnectToProcess(game)) => Some(Ok(game)),
                    Err(e) => Some(Err(e)),
                    _ => None,
                }
            },
        ),
    );

    
    let mut game_state = GameState::Stopped;
    let mut process_info = None;
    loop {
        if let Some(info) = process_info {
            let process = Process::open(info).ok().flatten();

            if let Some(process) = process {
                debug!("Process Opened");

                let global_injected = process.inject().ok().flatten();
                if let Some(injected) = global_injected.as_ref() {
                    debug!("Process Injected");

                    let mut timer_interval = tokio::time::interval(Duration::from_millis(10));
                    let mut animation_interval = tokio::time::interval(Duration::from_millis(50));

                    let mut timer = 0.0;
                    let mut animations = HashSet::new();
                    let mut last_timer_update = std::time::Instant::now();

                    loop {
                        tokio::select! {
                            _ = timer_interval.tick() => {

                                match injected.timer()
                                {
                                    Ok(new_timer) => {
                                        if new_timer < timer {
                                            message_bus.send(Message::TimerReset.into())?;

                                            last_timer_update = std::time::Instant::now();
                                        }

                                        if new_timer == timer {

                                            if game_state != GameState::Paused && std::time::Instant::now() - last_timer_update > Duration::from_millis(150) {
                                                message_bus.send(Message::GameStateChanged(GameState::Paused).into())?;
                                                game_state = GameState::Paused;
                                            }
                                        }else {
                                            if  game_state != GameState::Running {
                                                message_bus.send(Message::GameStateChanged(GameState::Running).into())?;
                                                game_state = GameState::Running;

                                            }
                                            last_timer_update = std::time::Instant::now();
                                        }
                                        timer = new_timer;
                                    },
                                    Err(_) => {
                                        if game_state != GameState::Stopped {
                                            message_bus.send(Message::GameStateChanged(GameState::Stopped).into())?;
                                        }
                                        break;
                                    },
                                }


                            }
                            _ = animation_interval.tick() => {
                                    let new_animations = injected.animation_list();

                                    if let Ok(new_animations) = new_animations {

                                        let changed = compate_animations(&animations, &new_animations);

                                        if !changed.is_empty()
                                        {
                                            message_bus.send(Message::AnimationsChanged(changed).into())?;
                                        }
                                        animations = new_animations;
                                    }

                                }
                            result = in_box.next() => {
                                let game = match result {
                                    Some(Ok(game)) => game,
                                    Some(Err(e)) => return Err(e.into()),
                                    None => return Ok(()),
                                };

                                let info = match game {
                                    crate::Game::SkyrimSE => SKYRIM_SE,
                                    crate::Game::SkyrimVR => SKYRIM_VR,
                                };

                                process_info = Some(info);

                            }
                        }
                    }
                }
            }
        }

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(1)) => {}
            result = in_box.next() => {
                let game = match result {
                    Some(Ok(game)) => game,
                    Some(Err(e)) => return Err(e.into()),
                    None => return Ok(()),
                };

                let info = match game {
                    crate::Game::SkyrimSE => SKYRIM_SE,
                    crate::Game::SkyrimVR => SKYRIM_VR,
                };

                process_info = Some(info);
            }
        }
    }
}
