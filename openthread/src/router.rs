//! FTD-specific Router and Network Management APIs
//!
//! This module provides functionality specific to Full Thread Devices (FTD),
//! including router management, child device tracking, and network data operations.
//!
//! NOTE: This module requires FTD libraries to be built. See BUILD_FTD_LIBS.md for instructions.

use crate::OtError;

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

// FTD operations module - contains actual C bindings when FTD libraries are available
#[cfg(feature = "ftd")]
mod ftd_ops {
    use super::*;
    use crate::ot;
    use crate::sys::{
        otChildInfo, otError_OT_ERROR_NONE, otError_OT_ERROR_NOT_FOUND, otInstance, otNeighborInfo,
        otNeighborInfoIterator, otRouterInfo, otThreadGetChildInfoByIndex,
        otThreadGetMaxAllowedChildren, otThreadGetMaxChildIpAddresses, otThreadGetNextNeighborInfo,
        otThreadGetRouterInfo, otThreadSetMaxAllowedChildren, otThreadSetMaxChildIpAddresses,
    };
    use core::mem::MaybeUninit;

    impl From<&otChildInfo> for ChildInfo {
        fn from(info: &otChildInfo) -> Self {
            Self {
                ext_address: u64::from_be_bytes(info.mExtAddress.m8),
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
                is_state_valid: info.mIsStateRestoring(),
            }
        }
    }

    impl From<&otNeighborInfo> for NeighborInfo {
        fn from(info: &otNeighborInfo) -> Self {
            Self {
                ext_address: u64::from_be_bytes(info.mExtAddress.m8),
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
                link_established: info.mIsChild(),
            }
        }
    }

    impl From<&otRouterInfo> for RouterInfo {
        fn from(info: &otRouterInfo) -> Self {
            Self {
                ext_address: u64::from_be_bytes(info.mExtAddress.m8),
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

    pub(crate) fn get_child_info_by_index(
        instance: *mut otInstance,
        index: u16,
    ) -> Result<ChildInfo, OtError> {
        let mut child_info = MaybeUninit::<otChildInfo>::uninit();

        let result =
            unsafe { otThreadGetChildInfoByIndex(instance, index, child_info.as_mut_ptr()) };

        if result == otError_OT_ERROR_NONE {
            Ok((&unsafe { child_info.assume_init() }).into())
        } else if result == otError_OT_ERROR_NOT_FOUND {
            Err(OtError::new(result))
        } else {
            ot!(result)?;
            unreachable!()
        }
    }

    pub(crate) fn get_neighbor_info_by_index(
        instance: *mut otInstance,
        index: u16,
    ) -> Result<NeighborInfo, OtError> {
        let mut neighbor_info = MaybeUninit::<otNeighborInfo>::uninit();
        let mut iterator: otNeighborInfoIterator = 0; // OT_NEIGHBOR_INFO_ITERATOR_INIT

        // Iterate through neighbors until we reach the requested index
        for _ in 0..=index {
            let result = unsafe {
                otThreadGetNextNeighborInfo(instance, &mut iterator, neighbor_info.as_mut_ptr())
            };

            if result != otError_OT_ERROR_NONE {
                return Err(OtError::new(result));
            }
        }

        Ok((&unsafe { neighbor_info.assume_init() }).into())
    }

    pub(crate) fn get_router_info(
        instance: *mut otInstance,
        router_id: u16,
    ) -> Result<RouterInfo, OtError> {
        let mut router_info = MaybeUninit::<otRouterInfo>::uninit();

        ot!(unsafe { otThreadGetRouterInfo(instance, router_id, router_info.as_mut_ptr()) })?;

        Ok((&unsafe { router_info.assume_init() }).into())
    }

    pub(crate) fn get_max_allowed_children(instance: *mut otInstance) -> u16 {
        unsafe { otThreadGetMaxAllowedChildren(instance) }
    }

    pub(crate) fn set_max_allowed_children(
        instance: *mut otInstance,
        max_children: u16,
    ) -> Result<(), OtError> {
        ot!(unsafe { otThreadSetMaxAllowedChildren(instance, max_children) })
    }

    pub(crate) fn get_max_child_ip_addresses(instance: *mut otInstance) -> u8 {
        unsafe { otThreadGetMaxChildIpAddresses(instance) }
    }

    pub(crate) fn set_max_child_ip_addresses(
        instance: *mut otInstance,
        max_ip_addresses: u8,
    ) -> Result<(), OtError> {
        ot!(unsafe { otThreadSetMaxChildIpAddresses(instance, max_ip_addresses) })
    }
}

// Stub implementations when FTD bindings are not available
#[cfg(not(feature = "ftd"))]
mod ftd_ops {
    use super::*;
    use crate::sys::otInstance;

    pub(crate) fn get_child_info_by_index(
        _instance: *mut otInstance,
        _index: u16,
    ) -> Result<ChildInfo, OtError> {
        Err(OtError::new(crate::sys::otError_OT_ERROR_NOT_IMPLEMENTED))
    }

    pub(crate) fn get_neighbor_info_by_index(
        _instance: *mut otInstance,
        _index: u16,
    ) -> Result<NeighborInfo, OtError> {
        Err(OtError::new(crate::sys::otError_OT_ERROR_NOT_IMPLEMENTED))
    }

    pub(crate) fn get_router_info(
        _instance: *mut otInstance,
        _router_id: u16,
    ) -> Result<RouterInfo, OtError> {
        Err(OtError::new(crate::sys::otError_OT_ERROR_NOT_IMPLEMENTED))
    }

    pub(crate) fn get_max_allowed_children(_instance: *mut otInstance) -> u16 {
        42
    }

    pub(crate) fn set_max_allowed_children(
        _instance: *mut otInstance,
        _max_children: u16,
    ) -> Result<(), OtError> {
        Err(OtError::new(crate::sys::otError_OT_ERROR_NOT_IMPLEMENTED))
    }

    pub(crate) fn get_max_child_ip_addresses(_instance: *mut otInstance) -> u8 {
        0
    }

    pub(crate) fn set_max_child_ip_addresses(
        _instance: *mut otInstance,
        _max_ip_addresses: u8,
    ) -> Result<(), OtError> {
        Err(OtError::new(crate::sys::otError_OT_ERROR_NOT_IMPLEMENTED))
    }
}

// Public API that delegates to the appropriate implementation
pub(crate) use ftd_ops::*;
