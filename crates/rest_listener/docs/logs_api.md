# logs_api

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
**Warg_FetchLogs**](logs_api.md#Warg_FetchLogs) | **POST** /logs/fetch | Fetches logs for a requested package.


# **Warg_FetchLogs**
> models::V1FetchLogsResponse Warg_FetchLogs(optional)
Fetches logs for a requested package.

NOTE: Current axios API uses /fetch/logs

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **optional** | **map[string]interface{}** | optional parameters | nil if no parameters

### Optional Parameters
Optional parameters are passed through a map[string]interface{}.

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **root_period_algo** | **String**|  | [default to "HASH_ALGORITHM_UNKNOWN".to_string()]
 **root_period_bytes** | **swagger::ByteArray**|  | 
 **operator_period_algo** | **String**|  | [default to "HASH_ALGORITHM_UNKNOWN".to_string()]
 **operator_period_bytes** | **swagger::ByteArray**|  | 

### Return type

[**models::V1FetchLogsResponse**](v1FetchLogsResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

