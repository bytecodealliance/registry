# warg_api

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
**Warg_FetchCheckpoint**](warg_api.md#Warg_FetchCheckpoint) | **POST** /checkpoint/fetch | Fetches logs for a root.
**Warg_ProveConsistency**](warg_api.md#Warg_ProveConsistency) | **POST** /prove/consistency | Proves consistency between an old root and a new one.
**Warg_ProveInclusion**](warg_api.md#Warg_ProveInclusion) | **POST** /prove/inclusion | Proves inclusion between a log and a map.


# **Warg_FetchCheckpoint**
> models::V1FetchCheckpointResponse Warg_FetchCheckpoint()
Fetches logs for a root.

NOTE: Current axios API uses /fetch/checkpoint

### Required Parameters
This endpoint does not need any parameter.

### Return type

[**models::V1FetchCheckpointResponse**](v1FetchCheckpointResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **Warg_ProveConsistency**
> models::V1ProveConsistencyResponse Warg_ProveConsistency(optional)
Proves consistency between an old root and a new one.

NOTE: Current axios API uses /proof/consistency

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **optional** | **map[string]interface{}** | optional parameters | nil if no parameters

### Optional Parameters
Optional parameters are passed through a map[string]interface{}.

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **old_root_period_algo** | **String**|  | [default to "HASH_ALGORITHM_UNKNOWN".to_string()]
 **old_root_period_bytes** | **swagger::ByteArray**|  | 
 **new_root_period_algo** | **String**|  | [default to "HASH_ALGORITHM_UNKNOWN".to_string()]
 **new_root_period_bytes** | **swagger::ByteArray**|  | 

### Return type

[**models::V1ProveConsistencyResponse**](v1ProveConsistencyResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **Warg_ProveInclusion**
> models::V1ProveInclusionResponse Warg_ProveInclusion(optional)
Proves inclusion between a log and a map.

NOTE: Current axios API uses /proof/inclusion

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **optional** | **map[string]interface{}** | optional parameters | nil if no parameters

### Optional Parameters
Optional parameters are passed through a map[string]interface{}.

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **checkpoint_period_log_root_period_algo** | **String**|  | [default to "HASH_ALGORITHM_UNKNOWN".to_string()]
 **checkpoint_period_log_root_period_bytes** | **swagger::ByteArray**|  | 
 **checkpoint_period_log_length** | **i64**|  | 
 **checkpoint_period_map_root_period_algo** | **String**|  | [default to "HASH_ALGORITHM_UNKNOWN".to_string()]
 **checkpoint_period_map_root_period_bytes** | **swagger::ByteArray**|  | 

### Return type

[**models::V1ProveInclusionResponse**](v1ProveInclusionResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

