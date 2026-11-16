// SPDX-FileCopyrightText: Copyright (c) 2025-2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! A compute driver that runs sandboxes as systemd units.
//!
//! Runs a sandbox as system units on the gateway's host. See the crate's
//! `README.md`.

pub mod config;
pub mod driver;
pub mod envfile;
pub mod grpc;
mod helper;
mod isolation;
pub mod protocol;

pub use config::SystemdComputeConfig;
pub use driver::SystemdComputeDriver;
pub use grpc::ComputeDriverService;
