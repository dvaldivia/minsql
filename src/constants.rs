// This file is part of MinSQL
// Copyright (c) 2019 MinIO, Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

// Server Defaults
pub const DEFAULT_SERVER_ADDRESS: &str = "0.0.0.0:9999";

// Smart Fields
pub const SF_IP: &str = "$ip";
pub const SF_EMAIL: &str = "$email";
pub const SF_DATE: &str = "$date";
pub const SF_QUOTED: &str = "$quoted";
pub const SF_URL: &str = "$url";
pub const SF_PHONE: &str = "$phone";
pub const SF_USER_AGENT: &str = "$user_agent";

pub const SMART_FIELDS_RAW_RE: &str =
    r"((\$(ip|email|date|url|quoted|phone|user_agent))([0-9]+)*)\b";

// MIME Types
pub const UNKNOWN_CONTENT_TYPE: &str = "text/plain";
pub const IMAGE_JPEG: &str = "image/jpeg";
pub const APP_JAVASCRIPT: &str = "application/javascript";
pub const APP_JSON: &str = "application/json";
pub const TEXT_HTML: &str = "text/html";

bitflags! {
    // ScanFlags determine which regex should be evaluated
    // If you are adding new values make sure to add the next power of 2 as
    // they are evaluated using a bitwise operation
    pub struct ScanFlags: u32 {
        const NONE = 1;
        const IP = 2;
        const EMAIL = 4;
        const DATE = 8;
        const QUOTED = 16;
        const URL = 32;
        const PHONE = 64;
        const USER_AGENT = 128;
    }
}
