# package_api

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
**Warg_GetPackage**](package_api.md#Warg_GetPackage) | **GET** /package/{packageId} | Used for polling while package is being in the processed of publishing.
**Warg_GetPackageRecord**](package_api.md#Warg_GetPackageRecord) | **GET** /package/{packageId}/records/{recordId} | Get a specific record within a package.
**Warg_PublishPackage**](package_api.md#Warg_PublishPackage) | **POST** /package | Request that a new package be published.


# **Warg_GetPackage**
> models::V1GetPackageResponse Warg_GetPackage(package_id)
Used for polling while package is being in the processed of publishing.

NOTE: This is a substitute for /package/{package_id}/pending/{record_id} which seemed superfluous.

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **package_id** | **String**| IDEA: Could add field mask to return more details like records. | 

### Return type

[**models::V1GetPackageResponse**](v1GetPackageResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **Warg_GetPackageRecord**
> models::V1Record Warg_GetPackageRecord(package_id, record_id)
Get a specific record within a package.

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **package_id** | **String**|  | 
  **record_id** | **String**|  | 

### Return type

[**models::V1Record**](v1Record.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **Warg_PublishPackage**
> models::V1PublishPackageResponse Warg_PublishPackage(optional)
Request that a new package be published.

NOTE: Current axios API has PublishRequest => PendingRecordResponse

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **optional** | **map[string]interface{}** | optional parameters | nil if no parameters

### Optional Parameters
Optional parameters are passed through a map[string]interface{}.

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **name** | **String**|  | 
 **record_period_contents** | **swagger::ByteArray**|  | 
 **record_period_key_id** | **String**|  | 
 **record_period_signature** | **String**|  | 

### Return type

[**models::V1PublishPackageResponse**](v1PublishPackageResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

