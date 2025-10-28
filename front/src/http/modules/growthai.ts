import createAxios from "@/http/axios";
import createAxiosApi from "@/http/axios_api";

export const prompts_search = (ns: string, query: object) => {
  let data = {
    uri: `object://${ns}/PromptTemplates#paged_query`,
    params: query
  }
  return createAxios({
    url: `/management/execute/paged`,
    method: 'POST',
    data: data
  })
};

export const prompts_save = (ns: string, query: object) => {
  let data = {
    uri: `object://${ns}/PromptTemplates#upsert`,
    params: query
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const prompts_delete = (ns: string, query: object) => {
  let data = {
    uri: `object://${ns}/PromptTemplates#delete`,
    params: query
  }
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const prompts_get = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/PromptTemplates#select`,
    params: {
      id: query
    }
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const prompts_retrieval_test = (ns: string, data: any) => {
  return createAxiosApi({
    url: `/api/rag/${ns}/anyone/test/prompt/retrieval`,
    method: 'POST',
    data
  })
};

export const memory_search = (ns: string, query: object) => {
  let data = {
    uri: `object://${ns}/Memories#paged_query`,
    params: query
  }
  return createAxios({
    url: `/management/execute/paged`,
    method: 'POST',
    data: data
  })
};

export const memory_save = (ns: string, query: object) => {
  let data = {
    uri: `object://${ns}/Memories#upsert`,
    params: query
  }
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const memory_delete = (ns: string, query: object) => {
  let data = {
    uri: `object://${ns}/Memories#delete`,
    params: query
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const memory_get = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/Memories#select`,
    params: { id: query }
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};


export const knowledge_get = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/KnowledgeBase#select`,
    params: query
  }
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const knowledge_query = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/KnowledgeBase#query`,
    params: query
  }
  return createAxios({
    url: `/management/execute/list`,
    method: 'POST',
    data
  })
};

export const knowledge_delete = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/KnowledgeBase#delete`,
    params: query
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const knowledge_save = (ns: string, query: object) => {
  let data = {
    uri: `object://${ns}/KnowledgeBase#upsert`,
    params: query
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};


export const chunkset_get = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/ChunkSet#select`,
    params: {
      id: query
    }
  }
  return createAxios({
    url: `/management/execute/option`,
    method: 'GET',
    data
  })
};

export const chunkset_query = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/ChunkSet#paged_query`,
    params: query
  }
  return createAxios({
    url: `/management/execute/paged`,
    method: 'POST',
    data
  })
};

export const chunkset_delete = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/ChunkSet#delete`,
    params: query
  }
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const chunkset_retrieval_test = (ns: string, data: any) => {  
  return createAxiosApi({
    url: `/api/rag/${ns}/anyone/test/rag/retrieval`,
    method: 'POST',
    data
  })
};

export const chunkset_save = (ns: string, query: object) => {
  let data = {
    uri: `object://${ns}/ChunkSet#upsert`,
    params: query
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};


export const docset_get = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/DocumentSet#select`,
    params: {
      id: query
    }
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const docset_query = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/DocumentSet#paged_query`,
    params: query
  }
  return createAxios({
    url: `/management/execute/paged`,
    method: 'POST',
    data
  })
};

export const docset_delete = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/DocumentSet#delete`,
    params: query
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const docset_save = (ns: string, query: object) => {
  let data = {
    uri: `object://${ns}/DocumentSet#upsert`,
    params: query
  }
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};


export const structset_get = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/Structsets#select`,
    params: { id: query }
  }

  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const structset_query = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/Structsets#paged_query`,
    params: query
  }
  return createAxios({
    url: `/management/execute/paged`,
    method: 'POST',
    data
  })
};

export const structset_delete = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/Structsets#delete`,
    params: query
  }
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const structset_save = (ns: string, query: object) => {
  let data = {
    uri: `object://${ns}/Structsets#upsert`,
    params: query
  }
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const toolcall_list = (ns: string, name: string, data: object) => {
  return createAxiosApi({
    url: `/api/rag/${ns}/${name}/list_all_tools`,
    method: 'POST',
    data
  })
};

export const toolcall_mcpids = (ns: string, name: string) => {
  return createAxiosApi({
    url: `/api/rag/${ns}/${name}/mcp_ids`,
    method: 'GET'
  })
};

export const toolcall_search = (ns: string, name: string, data: object) => {
  return createAxiosApi({
    url: `/api/rag/${ns}/${name}/list_all_tools`,
    method: 'POST',
    data
  })
};

export const toolcall_invoke = (ns: string, name: string, data: object) => {
  return createAxiosApi({
    url: `/api/rag/${ns}/${name}/invoke`,
    method: 'POST',
    data
  })
};

export const toolcall_refresh = (ns: string, name: string) => {
  return createAxiosApi({
    url: `/api/rag/${ns}/${name}/refresh`,
    method: 'POST',
    data: {}
  })
};

export const variable_query = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/VariableInfo#query`,
    params: query
  }
  return createAxios({
    url: `/management/execute/list`,
    method: 'POST',
    data
  })
};

export const variable_delete = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/VariableInfo#delete`,
    params: query
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const variable_save = (ns: string, query: object) => {
  let data = {
    uri: `object://${ns}/VariableInfo#upsert`,
    params: query
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};


export const intent_query = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/Intents#query`,
    params: query
  }  
  return createAxios({
    url: `/management/execute/list`,
    method: 'POST',
    data
  })
};

export const intent_delete = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/Intents#delete`,
    params: query
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const intent_save = (ns: string, query: object) => {
  let data = {
    uri: `object://${ns}/Intents#upsert`,
    params: query
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const intent_example_query = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/IntentExamples#paged_query`,
    params: query
  }
  return createAxios({
    url: `/management/execute/paged`,
    method: 'POST',
    data
  })
};

export const intent_example_delete = (ns: string, query: any) => {
  let data = {
    uri: `object://${ns}/IntentExamples#delete`,
    params: query
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};

export const intent_example_save = (ns: string, query: object) => {
  let data = {
    uri: `object://${ns}/IntentExamples#upsert`,
    params: query
  }  
  return createAxios({
    url: `/management/execute/option`,
    method: 'POST',
    data
  })
};
