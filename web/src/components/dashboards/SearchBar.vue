<template>
    <div>
      <q-bar class="q-pa-sm bg-white">
        <span class="text-subtitle2 text-weight-bold">SQL</span>
        <q-space />

        <q-toggle
          v-model="dashboardPanelData.layout.showCustomQuery"
          :label="t('Custom SQL')"
        />
        <q-btn-dropdown
          color="white"
          text-color="black"
          @click="onDropDownClick"
        />
      </q-bar>
    </div>
    <div class="row" v-if="showQuery">
      <!-- <div class="col">
        <q-input
          v-model="dashboardPanelData.data.query"
          filled
          clearable
          autogrow
          color="green-8"
          label="Please provide SQL Query for Visualization"
          hint="Press Apply Button to complete query preview"
          :disable="!dashboardPanelData.meta.stream.showCustomQuery"
        />
      </div> -->
      <div class="col">
        <query-editor
        ref="queryEditorRef"
        class="monaco-editor"
        v-model:query="dashboardPanelData.data.query"
        v-model:fields="dashboardPanelData.meta.stream.selectedStreamFields"
        v-model:functions="dashboardPanelData.meta.stream.functions"
        @run-query="searchData"
        :readOnly="!dashboardPanelData.layout.showCustomQuery"
        ></query-editor>
        <!-- @update-query="updateQueryValue" -->
        <div style="color: red;" class="q-mx-sm">{{ data.queryErrors.join(', ') }}&nbsp;</div>
      </div>
    </div>
</template>

<script lang="ts">
// @ts-nocheck
import { defineComponent, ref, watch, reactive, toRaw } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useQuasar } from "quasar";
import { Parser } from "node-sql-parser";

import QueryEditor from "./QueryEditor.vue";
import useDashboardPanelData from "../../composables/useDashboardPanel";

export default defineComponent({
  name: "ComponentSearchSearchBar",
  components: {
    QueryEditor,
  },
  emits: ["searchdata"],
  methods: {
    searchData() {
      if (this.searchdashboardPanelData.loading == false) {
        // this.searchObj.runQuery = true;
        this.$emit("searchdata");
      }
    },
  },
  setup() {
    // show the query box
    const showQuery = ref(true)

    const data = reactive({
      queryErrors: [] // custom query errors
    })

    const router = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const { dashboardPanelData } = useDashboardPanelData()
    const parser = new Parser();
    let streamName = "";

    // toggle show query view
    const onDropDownClick= () =>{
        showQuery.value = !showQuery.value
    }

    // Generate the query when the fields are updated
    watch(() => [
      dashboardPanelData.data.fields.stream, 
      dashboardPanelData.data.fields.x, 
      dashboardPanelData.data.fields.y,
      dashboardPanelData.data.fields.filter,
      dashboardPanelData.layout.showCustomQuery
    ], () => {

      // only continue if current mode is auto query generation
      if(!dashboardPanelData.layout.showCustomQuery){
        console.log("Updating query");
        
        // STEP 1: first check if there is at least 1 field selected
        if(dashboardPanelData.data.fields.x.length == 0 && dashboardPanelData.data.fields.y.length == 0) {
          dashboardPanelData.data.query = ""
          return;
        }

        // STEP 2: Now, continue if we have at least 1 field selected
        // merge the fields list
        let query = "SELECT "
        const fields = [...dashboardPanelData.data.fields.x, ...dashboardPanelData.data.fields.y]
        const filter = [...dashboardPanelData.data.fields?.filter]
        const array = fields.map((field, i) => {
          let selector = ""
          // TODO: add aggregator
          if(field.aggregationFunction) {
            selector += `${field.aggregationFunction}(${field.column})`
          } else {
            selector += `${field.column}`
          }
          selector += ` as ${field.label}${i==fields.length-1 ? ' ' : ', '}`
          return selector
        })
        query += array.join("")

        // now add from stream name
        query += ` FROM '${dashboardPanelData.data.fields.stream}' `

        query += filter.some((it)=> ((it.type == "list" && it.values?.length > 0) || (it.type == "condition" && it.operator != null && it.value != null && it.value != ''))) ? "WHERE " : ""
        const filterData = filter?.map((field, i)=>{
          let selectFilter = ""
            if(field.type == "list" && field.values?.length > 0){
              selectFilter += `${field.column} IN (${field.values.map(it => `'${it}'`).join(', ')})`
            }else if (field.type == "condition" && field.operator != null && field.value != null && field.value != ''){
              selectFilter += `${field.column} `
              switch(field.operator) {
                case "=":
                case "<>":
                case "<":
                case ">":
                case "<=":
                case ">=":
                  selectFilter += `${field.operator} ${field.value}`
                  break;
                case "Contains":
                  selectFilter += `LIKE '%${field.value}%'`
                  break;
                case "Not Contains":
                  selectFilter += `NOT LIKE '%${field.value}%'`
                  break;
                default:
                  selectFilter += `${field.operator} ${field.value}`
                  break;
              }
            }
            return selectFilter
        })
        console.log("query: filterData",filterData);
        
        query += filterData.filter((it: any)=> it).join(" AND ")

        // add group by statement
        query += ` GROUP BY ${dashboardPanelData.data.fields.x[0]?.label}`

        console.log('generated query: ', query)

        dashboardPanelData.data.query = query
      }
    }, {deep: true})

    watch(() => dashboardPanelData.data.query, ()=>{
      console.log("query changes in search bar",dashboardPanelData.layout.showCustomQuery);

      // only continue if current mode is show custom query
      if(dashboardPanelData.layout.showCustomQuery){
        updateQueryValue()
      }
    })

    //TODO: verification to perform when the query is updated
    const updateQueryValue = () => {
      // store the query in the dashboard panel data
      // dashboardPanelData.meta.editorValue = value;
      // dashboardPanelData.data.query = value;

      if (dashboardPanelData.layout.showCustomQuery) {
        console.log("query: value", dashboardPanelData.data.query);

        // empty the errors
        data.queryErrors = []
        console.log("----------", dashboardPanelData.data.query);
        console.log("---sql mode-------", dashboardPanelData.layout.showCustomQuery);
        
        // Get the parsed query
        try {
          dashboardPanelData.meta.parsedQuery = parser.astify(dashboardPanelData.data.query);
          console.log(dashboardPanelData.meta.parsedQuery)
        } catch(e) {
          console.log("error")
          console.log(e)
          // exit as there is an invalid query
          data.queryErrors.push("Invalid SQL Syntax")
          return null;
        }
        console.log("--parsed query---", dashboardPanelData.meta.parsedQuery);
        
        if(!dashboardPanelData.meta.parsedQuery) {
          return;
        }

        // We have the parsed query, now get the columns and tables
        // get the columns first
        if(Array.isArray(dashboardPanelData.meta.parsedQuery?.columns) 
            && dashboardPanelData.meta.parsedQuery?.columns?.length > 0) {
          dashboardPanelData.meta.stream.customQueryFields = []
          dashboardPanelData.meta.parsedQuery.columns.forEach((item: any, index: any) => {
            let val;
            // if there is a lable, use that, else leave it
            if (item["as"] === undefined || item["as"] === null) {
              val = item["expr"]["column"];
            } else {
              val = item["as"];
            }
            if (!dashboardPanelData.meta.stream.customQueryFields.find(it => it.name == val)) {
              dashboardPanelData.meta.stream.customQueryFields.push({name: val, type: ''});
            }
          });
        } else {
          data.queryErrors.push("Invalid Columns")
        }

        // now check if the correct stream is selected
        if (dashboardPanelData.meta.parsedQuery.from?.length > 0) {
          console.log("---parsedQuery.from--------",dashboardPanelData.meta.parsedQuery.from);
    
          const streamFound = dashboardPanelData.meta.stream.streamResults.find(it => it.name == dashboardPanelData.meta.parsedQuery.from[0].table)
          if(streamFound) {
            if(dashboardPanelData.data.fields.stream != streamFound.name) {
              dashboardPanelData.data.fields.stream = streamFound.name
            }
          } else {
            data.queryErrors.push("Invalid stream")
          }

          // if (dashboardPanelData.meta.parsedQuery.from[0].table !== dashboardPanelData.data.fields.stream) {
          //   let streamFound = false;
          //   const streamNameFromQuery = dashboardPanelData.meta.parsedQuery.from[0].table;

          //   dashboardPanelData.meta.stream.streamResults.forEach((stream) => {
          //     console.log("Query: get stream data from streamResults", stream);
              
          //     if (stream.name == streamNameFromQuery) {
          //       streamFound = true;
          //       let itemObj = {
          //         label: stream.name,
          //         value: stream.name,
          //       };
          //       dashboardPanelData.data.stream.selectedStream = itemObj;
          //       stream.schema.forEach((field) => {
          //         dashboardPanelData.meta.stream.selectedStreamFields.push({
          //           name: field.name,
          //         });
          //       });
          //     }
          //   });

          //   if (streamFound == false) {
          //     dashboardPanelData.data.stream.selectedStream = {};
          //     dashboardPanelData.meta.stream.selectedStreamFields = [];
          //     $q.notify({
          //       message: "Stream not found",
          //       color: "negative",
          //       position: "top",
          //       timeout: 2000,
          //     });
          //   }
          // }
        } else {
          data.queryErrors.push("Stream name required")
        }
      }

    };


    return {
      t,
      router,
      updateQueryValue,
      onDropDownClick,
      showQuery,
      dashboardPanelData,
      data
    };
  },
});
</script>

