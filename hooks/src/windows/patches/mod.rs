use frida_gum::{Gum, Module};
use shared_types::HookError;

mod find_first_file;
mod nt_create_file;
mod nt_open_file;
mod nt_query_directory_file;
mod nt_query_object;

pub(crate) use find_first_file::*;
pub(crate) use nt_create_file::*;
pub(crate) use nt_open_file::*;
pub(crate) use nt_query_directory_file::*;
pub(crate) use nt_query_object::*;

pub(crate) type FuncPatcher = fn(&Gum, &Module, &str) -> Result<(), HookError>;

pub static WIN32_TARGETS: [(&'static str, Option<FuncPatcher>); 34] = [
  ("GetFileAttributesExA", None),
  ("GetFileAttributesA", None),
  ("GetFileAttributesExW", None),
  ("GetFileAttributesW", None),
  ("SetFileAttributesW", None),
  ("CreateDirectoryW", None),
  ("RemoveDirectoryW", None),
  ("DeleteFileW", None),
  ("GetCurrentDirectoryA", None),
  ("GetCurrentDirectoryW", None),
  ("SetCurrentDirectoryA", None),
  ("SetCurrentDirectoryW", None),
  ("ExitProcess", None),
  ("CreateProcessInternalW", None),
  ("MoveFileA", None),
  ("MoveFileW", None),
  ("MoveFileExA", None),
  ("MoveFileExW", None),
  ("MoveFileWithProgressA", None),
  ("MoveFileWithProgressW", None),
  ("CopyFileExW", None),
  ("GetPrivateProfileStringA", None),
  ("GetPrivateProfileStringW", None),
  ("GetPrivateProfileSectionA", None),
  ("GetPrivateProfileSectionW", None),
  ("WritePrivateProfileStringA", None),
  ("WritePrivateProfileStringW", None),
  ("GetFullPathNameA", None),
  ("GetFullPathNameW", None),
  ("FindFirstFileExW", Some(find_first_file_ex_w)),
  ("LoadLibraryExA", None),
  ("LoadLibraryExW", None),
  ("GetModuleFileNameA", None),
  ("GetModuleFileNameW", None),
];

pub static WIN8_PLUS_WIN32_TARGETS: [(&'static str, Option<FuncPatcher>); 1] =
  [("CopyFile2", None)];

pub static NT_TARGETS: [(&'static str, Option<FuncPatcher>); 11] = [
  ("NtQueryFullAttributesFile", None),
  ("NtQueryAttributesFile", None),
  ("NtQueryDirectoryFile", Some(nt_query_directory_file)),
  ("NtQueryDirectoryFileEx", Some(nt_query_directory_file_ex)),
  ("NtQueryObject", Some(nt_query_object)),
  ("NtQueryInformationFile", None),
  ("NtQueryInformationByName", None),
  ("NtOpenFile", Some(nt_open_file)),
  ("NtCreateFile", Some(nt_create_file)),
  ("NtClose", None),
  ("NtTerminateProcess", None),
];
