use std::{ffi::c_void, mem::size_of};

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

#[derive(Debug, Clone, Copy)]
pub struct ProcessInfo {
    process_name: &'static str,
    pattern: &'static [u8],
}

pub static SKYRIM_SE: ProcessInfo = ProcessInfo {
    process_name: "SkyrimSE.exe",
    pattern: &[
        0x48, 0x8B, 0xC4, 0x57, 0x48, 0x81, 0xEC, 0x40, 0x01, 0x00, 0x00, 0x48, 0xC7, 0x44, 0x24,
        0x20, 0xFE, 0xFF, 0xFF, 0xFF, 0x48, 0x89, 0x58, 0x10, 0x48, 0x89, 0x70, 0x18,
    ],
};

type Error = u32;

#[derive(Debug)]
pub struct Process {
    handle: *mut c_void,
    pub pid: u32,
    base_address: *mut u8,
    pattern: &'static [u8],
}

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
                    println!("handle is null");
                    return Err(winapi::um::errhandlingapi::GetLastError());
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
                    println!("snapshot is null");
                    return Err(winapi::um::errhandlingapi::GetLastError());
                }

                let main_module = Module32First(snapshot, &mut entry as *mut MODULEENTRY32);
                if main_module == 0 {
                    println!("get main module error");
                    return Err(winapi::um::errhandlingapi::GetLastError());
                }

                Ok(Some(Self {
                    handle,
                    pid,
                    base_address: entry.modBaseAddr,
                    pattern: info.pattern,
                }))
            }
        } else {
            Ok(None)
        }
    }

    #[tracing::instrument]
    fn read_raw_bytes(
        &mut self,
        address: *mut u8,
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
                Err(unsafe { winapi::um::errhandlingapi::GetLastError() })
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
                Err(unsafe { winapi::um::errhandlingapi::GetLastError() })
            } else {
                Ok(())
            },
        )
    }

    #[tracing::instrument]
    fn read<T>(&mut self, address: *mut u8) -> Result<T, Error>
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

    fn write<T>(&mut self, address: *mut u8, data: &mut T) -> Result<(), Error>
    where
        T: Default,
    {
        let len = std::mem::size_of::<T>();

        let (bytes_read, res) = self.write_raw_bytes(address, data as *mut _ as *mut u8, len);

        let _ = res?;

        if bytes_read as usize != len {
            //err
            todo!()
        } else {
            Ok(())
        }
    }

    #[tracing::instrument]
    fn read_slice(&mut self, address: *mut u8, slice: &mut [u8]) -> Result<i32, Error> {
        let (bytes_read, result) =
            self.read_raw_bytes(address, slice as *mut _ as *mut u8, slice.len());

        match result {
            Ok(_) => Ok(bytes_read),
            Err(299) => Ok(bytes_read),
            Err(e) => Err(e),
        }
    }

    #[tracing::instrument]
    fn write_slice(&mut self, address: *mut u8, slice: &mut [u8]) -> Result<i32, Error> {
        let (bytes_read, result) =
            self.write_raw_bytes(address, slice as *mut _ as *mut u8, slice.len());

        match result {
            Ok(_) => Ok(bytes_read),
            Err(299) => Ok(bytes_read),
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
            debug!(?mbi.BaseAddress);
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

        Err(0)
    }

    pub fn hook(
        &mut self,
        hook: *mut u8,
        func: *mut u8,
        len: usize,
    ) -> Result<(), Error> {
        let mut oldProtect = 0u32;
        let ok = unsafe {
            VirtualProtectEx(
                self.handle,
                hook as *mut _,
                len,
                PAGE_READWRITE,
                &mut oldProtect as *mut _,
            )
        };

        if ok == 0 {
            return Err(unsafe { winapi::um::errhandlingapi::GetLastError() });
        }

        let mut empty_bytes = Vec::with_capacity(len);
        empty_bytes.resize(len, 0x90u8);

        self.write_slice(hook, &mut empty_bytes)?;

        let byte_jump_delta = (func as isize - hook as isize - 5).to_le_bytes();

        let mut managed_array = [
            0xE9,
            byte_jump_delta[0],
            byte_jump_delta[1],
            byte_jump_delta[2],
            byte_jump_delta[3],
        ];

        self.write_slice(hook, &mut managed_array)?;

        let ok = unsafe {
            VirtualProtectEx(
                self.handle,
                hook as *mut _,
                len,
                oldProtect,
                &mut oldProtect as *mut _,
            )
        };

        if ok == 0 {
            return Err(unsafe { winapi::um::errhandlingapi::GetLastError() });
        }

        Ok(())
    }

    #[tracing::instrument()]
    pub fn inject(&mut self) -> Result<(), u32> {
        if let Ok(Some(ptr)) = self.find_pattern() {
            let ptr = unsafe { ptr.offset(0x1c) };

            let data_offset = 0x100;
            let program = [
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
                0xEB,
                0xE8, //jmp increaseArray                             <-createNewArrayItem
                0x48, 0x89,
                0x38, //                                              <-countUpArrayItem
                0x48, 0x83, 0xC0, 0x08, //
                0x48, 0x8B, 0x08, //
                0x48, 0x83, 0xC1, 0x01, //
                0x48, 0x89, 0x08, //
                0x58, //pop rax                                       <-finishUp
                0x59, //pop rcx
                0xE9, 0x28, 0x1C, 0x1B,
                0x00, //jmp INJECT                INJECT
                      //                                              <-randomData
            ];
            let program_len = program.len();

            let check = self.read::<u8>(ptr)?;

            debug!(check);

            let data_ptr = if check == 0xE9 {
                let ptr_plus1 = unsafe { ptr.offset(0x01) };

                let offset = self.read::<u32>(ptr_plus1)?;

                unsafe { ptr.offset(offset as isize + 5 + program_len as isize + data_offset) }
            } else {
                let ptr_function = self.allocate_memory(ptr, 10000)? as *mut u8;
                let ptr_data = unsafe { ptr_function.offset(program_len as isize) };

                let mut bytes = [0u8; 10000];

                bytes[0..program_len].copy_from_slice(&program);

                for (offset, b) in (self.base_address as usize)
                    .to_le_bytes()
                    .iter()
                    .enumerate()
                {
                    bytes[0x0A + offset] = *b;
                }

                for (offset, b) in (ptr_data as usize).to_le_bytes().iter().enumerate() {
                    bytes[0x4C + offset] = *b;
                }

                let address_bytes = (ptr as usize + 6 - ptr_function as usize - 0x8A).to_le_bytes();
                dbg!(ptr, ptr, ptr_function, address_bytes);
                for (offset, b) in address_bytes.iter().enumerate() {
                    bytes[0x86 + offset] = *b;
                }

                self.write_slice(ptr_function, &mut bytes)?;

                self.hook(ptr, ptr_function, 6)?;

                ptr_data.wrapping_offset(0x100)
            };

            dbg!(data_ptr);
        };

        Ok(())
    }

    #[tracing::instrument]
    fn find_pattern(&mut self) -> Result<Option<*mut u8>, u32> {
        let mut buffer: [u8; 1024] = [0; 1024];
        let mut next: [u8; 1024] = [0; 1024];

        let mut offset = 0;

        let mut bytes_read = 0;

        unsafe {
            bytes_read = self.read_slice(self.base_address.offset(offset), &mut buffer)? as usize;
        }

        while bytes_read == 1024 {
            let bytes_read = self.read_slice(
                unsafe { self.base_address.offset(offset + 1024) },
                &mut next,
            )? as usize;

            for i in 0..1024 {
                let mut all_matched = true;
                for (matched_index, p) in self.pattern.iter().enumerate() {
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
                    return Ok(Some(unsafe { self.base_address.offset(offset) }));
                }

                offset += 1;
            }

            buffer = next;
        }

        Ok(None)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            winapi::um::handleapi::CloseHandle(self.handle);
        }
    }
}
