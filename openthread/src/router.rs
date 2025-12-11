//! FTD-specific Router and Network Management APIs
//!
//! This module provides functionality specific to Full Thread Devices (FTD),
//! including router management, child device tracking, and network data operations.

use core::mem::MaybeUninit;

use crate::sys::{
    otChildInfo, otError_OT_ERROR_NONE, otError_OT_ERROR_NOT_FOUND, otInstance,
    otNeighborInfo, otRouterInfo, otThreadGetChildInfoByIndex, otThreadGetMaxAllowedChildren,
    otThreadGetMaxChildIpAddresses, otThreadGetNeighborInfoByIndex, otThreadGetRouterInfo,
    otThreadSetMaxAllowedChildren, otThreadSetMaxChildIpAddresses,
};
use crate::{ot, OtError};

/// Information about a child device connected to this router
#[derive(Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChildInfo {
    /// Extended address of the child
    pub ext_address: u64,
    /// Timeout in seconds
    pub timeout: u32,
    /// Age in seconds
    pub age: u32,
    /// RLOC16 address
    pub rloc16: u16,
    /// Child ID (RLOC16 & 0x1FF)
    pub child_id: u16,
    /// Network data version
    pub network_data_version: u8,
    /// Link quality in
    pub link_quality_in: u8,
    /// Average RSSI
    pub average_rssi: i8,
    /// Last RSSI
    pub last_rssi: i8,
    /// Frame error rate (0-100)
    pub frame_error_rate: u16,
    /// Message error rate (0-100)
    pub message_error_rate: u16,
    /// Is the child an RX-on-when-idle device
    pub rx_on_when_idle: bool,
    /// Is the child a full thread device
    pub full_thread_device: bool,
    /// Is the child a full network data device
    pub full_network_data: bool,
    /// Is link established
    pub is_state_valid: bool,
}

impl From<&otChildInfo> for ChildInfo {
    fn from(info: &otChildInfo) -> Self {
        Self {
            ext_address: u64::from_be_bytes(unsafe { info.mExtAddress.m8 }),
            timeout: info.mTimeout,
            age: info.mAge,
            rloc16: info.mRloc16,
            child_id: info.mChildId,
            network_data_version: info.mNetworkDataVersion,
            link_quality_in: info.mLinkQualityIn,
            average_rssi: info.mAverageRssi,
            last_rssi: info.mLastRssi,
            frame_error_rate: info.mFrameErrorRate,
            message_error_rate: info.mMessageErrorRate,
            rx_on_when_idle: info.mRxOnWhenIdle(),
            full_thread_device: info.mFullThreadDevice(),
            full_network_data: info.mFullNetworkData(),
            is_state_valid: info.mIsStateValid(),
        }
    }
}

/// Information about a neighboring router
#[derive(Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NeighborInfo {
    /// Extended address
    pub ext_address: u64,
    /// Age in seconds
    pub age: u32,
    /// RLOC16 address
    pub rloc16: u16,
    /// Link frame counter
    pub link_frame_counter: u32,
    /// MLE frame counter
    pub mle_frame_counter: u32,
    /// Link quality in
    pub link_quality_in: u8,
    /// Average RSSI
    pub average_rssi: i8,
    /// Last RSSI
    pub last_rssi: i8,
    /// Frame error rate (0-100)
    pub frame_error_rate: u16,
    /// Message error rate (0-100)
    pub message_error_rate: u16,
    /// Is the neighbor an RX-on-when-idle device
    pub rx_on_when_idle: bool,
    /// Is the neighbor a full thread device
    pub full_thread_device: bool,
    /// Is the neighbor a full network data device
    pub full_network_data: bool,
    /// Is link established
    pub link_established: bool,
}

impl From<&otNeighborInfo> for NeighborInfo {
    fn from(info: &otNeighborInfo) -> Self {
        Self {
            ext_address: u64::from_be_bytes(unsafe { info.mExtAddress.m8 }),
            age: info.mAge,
            rloc16: info.mRloc16,
            link_frame_counter: info.mLinkFrameCounter,
            mle_frame_counter: info.mMleFrameCounter,
            link_quality_in: info.mLinkQualityIn,
            average_rssi: info.mAverageRssi,
            last_rssi: info.mLastRssi,
            frame_error_rate: info.mFrameErrorRate,
            message_error_rate: info.mMessageErrorRate,
            rx_on_when_idle: info.mRxOnWhenIdle(),
            full_thread_device: info.mFullThreadDevice(),
            full_network_data: info.mFullNetworkData(),
            link_established: info.mIsChildIdValid(),
        }
    }
}

/// Information about a router in the network
#[derive(Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RouterInfo {
    /// Extended address
    pub ext_address: u64,
    /// RLOC16 address
    pub rloc16: u16,
    /// Router ID
    pub router_id: u8,
    /// Next hop to router
    pub next_hop: u8,
    /// Path cost to router
    pub path_cost: u8,
    /// Link quality in
    pub link_quality_in: u8,
    /// Link quality out
    pub link_quality_out: u8,
    /// Age in seconds
    pub age: u8,
    /// Is the router allocated
    pub allocated: bool,
    /// Is link established
    pub link_established: bool,
}

impl From<&otRouterInfo> for RouterInfo {
    fn from(info: &otRouterInfo) -> Self {
        Self {
            ext_address: u64::from_be_bytes(unsafe { info.mExtAddress.m8 }),
            rloc16: info.mRloc16,
            router_id: info.mRouterId,
            next_hop: info.mNextHop,
            path_cost: info.mPathCost,
            link_quality_in: info.mLinkQualityIn,
            link_quality_out: info.mLinkQualityOut,
            age: info.mAge,
            allocated: info.mAllocated(),
            link_established: info.mLinkEstablished(),
        }
    }
}

/// FTD-specific router management operations
pub(crate) struct RouterOps;

impl RouterOps {
    /// Get information about a child device by index
    ///
    /// # Arguments
    /// * `instance` - OpenThread instance
    /// * `index` - Child index
    ///
    /// # Returns
    /// Child information if found, None otherwise
    pub(crate) fn get_child_info_by_index(
        instance: *mut otInstance,
        index: u16,
    ) -> Result<ChildInfo, OtError> {
        let mut child_info = MaybeUninit::<otChildInfo>::uninit();

        let result = unsafe {
            otThreadGetChildInfoByIndex(instance, index, child_info.as_mut_ptr())
        };

        if result == otError_OT_ERROR_NONE {
            Ok(unsafe { child_info.assume_init() }.into())
        } else if result == otError_OT_ERROR_NOT_FOUND {
            Err(OtError::new(result))
        } else {
            ot!(result)?;
            unreachable!()
        }
    }

    /// Get information about a neighboring device by index
    ///
    /// # Arguments
    /// * `instance` - OpenThread instance
    /// * `index` - Neighbor index
    ///
    /// # Returns
    /// Neighbor information if found, None otherwise
    pub(crate) fn get_neighbor_info_by_index(
        instance: *mut otInstance,
        index: u16,
    ) -> Result<NeighborInfo, OtError> {
        let mut neighbor_info = MaybeUninit::<otNeighborInfo>::uninit();

        let result = unsafe {
            otThreadGetNeighborInfoByIndex(instance, index, neighbor_info.as_mut_ptr())
        };

        if result == otError_OT_ERROR_NONE {
            Ok(unsafe { neighbor_info.assume_init() }.into())
        } else if result == otError_OT_ERROR_NOT_FOUND {
            Err(OtError::new(result))
        } else {
            ot!(result)?;
            unreachable!()
        }
    }

    /// Get information about a router
    ///
    /// # Arguments
    /// * `instance` - OpenThread instance
    /// * `router_id` - Router ID
    ///
    /// # Returns
    /// Router information if found
    pub(crate) fn get_router_info(
        instance: *mut otInstance,
        router_id: u16,
    ) -> Result<RouterInfo, OtError> {
        let mut router_info = MaybeUninit::<otRouterInfo>::uninit();

        ot!(unsafe { otThreadGetRouterInfo(instance, router_id, router_info.as_mut_ptr()) })?;

        Ok(unsafe { router_info.assume_init() }.into())
    }

    /// Get the maximum number of children allowed
    pub(crate) fn get_max_allowed_children(instance: *mut otInstance) -> u16 {
        unsafe { otThreadGetMaxAllowedChildren(instance) }
    }

    /// Set the maximum number of children allowed
    ///
    /// # Arguments
    /// * `instance` - OpenThread instance
    /// * `max_children` - Maximum number of children
    pub(crate) fn set_max_allowed_children(
        instance: *mut otInstance,
        max_children: u16,
    ) -> Result<(), OtError> {
        ot!(unsafe { otThreadSetMaxAllowedChildren(instance, max_children) })
    }

    /// Get the maximum number of IP addresses per child
    pub(crate) fn get_max_child_ip_addresses(instance: *mut otInstance) -> u8 {
        unsafe { otThreadGetMaxChildIpAddresses(instance) }
    }

    /// Set the maximum number of IP addresses per child
    ///
    /// # Arguments
    /// * `instance` - OpenThread instance
    /// * `max_ip_addresses` - Maximum number of IP addresses per child
    pub(crate) fn set_max_child_ip_addresses(
        instance: *mut otInstance,
        max_ip_addresses: u8,
    ) -> Result<(), OtError> {
        ot!(unsafe { otThreadSetMaxChildIpAddresses(instance, max_ip_addresses) })
    }
}
