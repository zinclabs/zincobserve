<template>
  <div class="column index-menu">
    <div>
      <q-select
        v-model="dashboardPanelData.data.fields.stream"
        :label="
          dashboardPanelData.data.fields.stream ? '' : t('search.selectIndex')
        "
        :options="data.indexOptions"
        data-cy="index-dropdown"
        input-debounce="0"
        behavior="menu"
        filled
        borderless
        dense
      >
        <template #no-option>
          <q-item>
            <q-item-section> {{ t("search.noResult") }}</q-item-section>
          </q-item>
        </template>
      </q-select>
    </div>
    <div class="index-table q-mt-xs">
      <q-table
        :columns="[
          {
            name: 'name',
            field: 'name',
            align: 'left',
            label: 'Field',
            sortable: true,
          },
        ]"
        :rows="data.currentFieldsList"
        row-key="name"
        :filter="dashboardPanelData.meta.stream.filterField"
        :filter-method="filterFieldFn"
        :pagination="{ rowsPerPage: 10000 }"
        hide-header
        hide-bottom
        id="fieldList"
      >
        <template #body-cell-name="props">
          <q-tr :props="props">
            <q-td
              class="field_list"
              :class="
                dashboardPanelData.data.fields.x.find((it: any)=>it.column === props.row.name) ||
                dashboardPanelData.data.fields.y.find((it: any)=>it.column === props.row.name) ||
                dashboardPanelData.data.fields.filter.find((it: any)=>it.column === props.row.name)
                ? 'selected'
                : ''
              "
              :props="props"
            >
              <div class="field_overlay">
                <div class="field_label">
                  {{ props.row.name }}
                </div>
                <div class="field_icons">
                  <q-btn
                    color="white"
                    padding="sm"
                    text-color="black"
                    :disabled="dashboardPanelData.data.fields.x.length >= 1 ? true : false"
                    @click="addXAxisItem(props.row.name)"
                  >
                    <div>{{ dashboardPanelData.data.type != 'h-bar' ? '+X' : '+Y' }}</div>
                  </q-btn>
                  <q-btn
                    color="white"
                    padding="sm"
                    text-color="black"
                    :disabled="dashboardPanelData.data.fields.y.length >= 1 && dashboardPanelData.data.type == 'pie'  ? true : false"
                    @click="addYAxisItem(props.row.name)"
                  >
                    <div>{{ dashboardPanelData.data.type != 'h-bar' ? '+Y' : '+X' }}</div>
                  </q-btn>
                  <q-btn
                    color="white"
                    padding="sm"
                    text-color="black"
                    @click="addFilteredItem(props.row.name)"
                  >
                    <div>+F</div>
                  </q-btn>
                </div>
              </div>
            </q-td>
          </q-tr>
        </template>
        <template #top-right>
          <q-input
            v-model="dashboardPanelData.meta.stream.filterField"
            data-cy="index-field-search-input"
            filled
            borderless
            dense
            clearable
            debounce="1"
            :placeholder="t('search.searchField')"
          >
            <template #prepend>
              <q-icon name="search" />
            </template>
          </q-input>
        </template>
      </q-table>
    </div>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  reactive,
  ref,
  onMounted,
  computed,
  watch,
  onActivated,
} from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import { useRouter } from "vue-router";
import useDashboardPanelData from "../../../composables/useDashboardPanel";
import IndexService from "../../../services/index";
import queryService from "../../../services/nativequery";

export default defineComponent({
  name: "ComponentSearchIndexSelect",
  props: ["selectedXAxisValue", "selectedYAxisValue"],
  emits: ["update:selectedXAxisValue", "update:selectedYAxisValue"],
  setup() {
    const store = useStore();
    const router = useRouter();
    const { t } = useI18n();
    const data = reactive({
      schemaList: [],
      indexOptions: [],
      currentFieldsList: [],
    });
    const $q = useQuasar();
    const { dashboardPanelData, addXAxisItem, addYAxisItem } =
      useDashboardPanelData();
    const x = ref();
    const y = ref();
    console.log("schemaList:", data.schemaList);
    // console.log("schemaList name:", schemaList.data  );
    console.log("indexOptions:", data.indexOptions);

    onActivated(() => {
      console.log("inside mounted");
      getStreamList();
    });

    // update the selected stream fields list
    watch(
      () => [data.schemaList, dashboardPanelData.data.fields.stream],
      () => {
        const fields: any = data.schemaList.find(
          (it: any) => it.name == dashboardPanelData.data.fields.stream
        );
        dashboardPanelData.meta.stream.selectedStreamFields =
          fields?.schema || [];
      }
    );

    // update the current list fields if any of the lists changes
    watch(
      () => [
        dashboardPanelData.meta.stream.selectedStreamFields,
        dashboardPanelData.meta.stream.customQueryFields,
      ],
      () => {
        console.log("updated custom query fields or selected stream fields");

        data.currentFieldsList = [];
        data.currentFieldsList = [
          ...dashboardPanelData.meta.stream.customQueryFields,
          ...dashboardPanelData.meta.stream.selectedStreamFields,
        ];
      }
    );

    // get the stream list by making an API call
    const getStreamList = () => {
      IndexService.nameList(
        store.state.selectedOrganization.identifier,
        "",
        true
      ).then((res) => {
        data.schemaList = res.data.list;
        dashboardPanelData.meta.stream.streamResults = res.data.list;
        data.indexOptions = res.data.list.map((data: any) => {
          return data.name;
        });

        // set the first stream as the selected stream when the api loads the data
        if (
          !dashboardPanelData.data.fields.stream &&
          data.indexOptions.length > 0
        ) {
          dashboardPanelData.data.fields.stream = data.indexOptions[0];
        }
      });
    };

    const filterFieldFn = (rows: any, terms: any) => {
      var filtered = [];
      if (terms != "") {
        terms = terms.toLowerCase();
        for (var i = 0; i < rows.length; i++) {
          if (rows[i]["name"].toLowerCase().includes(terms)) {
            filtered.push(rows[i]);
          }
        }
      }
      return filtered;
    };

    const addFilteredItem = (name: string) => {
      console.log("name=", name);
      if (!dashboardPanelData.data.fields.filter) {
        dashboardPanelData.data.fields.filter = [];
      }

      if (
        !dashboardPanelData.data.fields.filter.find(
          (it: any) => it.column == name
        )
      ) {
        console.log("data");

        dashboardPanelData.data.fields.filter.push({
          type: "list",
          values: [],
          column: name,
          operator: null,
          value: null,
        });
      }

      if (!dashboardPanelData.meta.filterValue) {
        dashboardPanelData.meta.filterValue = [];
      }

      if (
        !dashboardPanelData.meta.filterValue.find(
          (it: any) => it.column == name
        )
      ) {
        let queryData = "SELECT ";

        // get unique value of the selected fields
        queryData += `${name} as value`;

        //now add the selected stream
        queryData += ` FROM '${dashboardPanelData.data.fields.stream}'`;

        console.log("queryData= ", queryData);
        // add group by statement
        queryData += ` GROUP BY value`;

        const query = {
          query: { sql: queryData, sql_mode: "full" },
        };

        queryService
          .runquery(query, store.state.selectedOrganization.identifier)
          .then((res) => {
            console.log(
              "-distinct vals--",
              res.data.hits.map((it: any) => it.value).filter((it: any) => it)
            );

            dashboardPanelData.meta.filterValue.push({
              column: name,
              value: res.data.hits
                .map((it: any) => it.value)
                .filter((it: any) => it),
            });

          })
          .catch((error) => {
            $q.notify({
              type: "negative",
              message: "Something went wrong!",
              timeout: 5000,
            });
          });
      }
    };

    return {
      t,
      store,
      router,
      filterFieldFn,
      addXAxisItem,
      addYAxisItem,
      addFilteredItem,
      data,
      getStreamList,
      dashboardPanelData,
    };
  },
});
</script>

<style lang="scss" scoped>
.q-menu {
  box-shadow: 0px 3px 15px rgba(0, 0, 0, 0.1);
  transform: translateY(0.5rem);
  border-radius: 0px;

  .q-virtual-scroll__content {
    padding: 0.5rem;
  }
}
.index-menu {
  width: 100%;
  height: 100%;

  .q-field {
    &__control {
      height: 35px;
      padding: 0px 5px;
      min-height: auto !important;

      &-container {
        padding-top: 0px !important;
      }
    }
    &__native :first-of-type {
      padding-top: 0.25rem;
    }
  }

  .q-select {
    text-transform: capitalize;
  }

  .index-table {
    width: 100%;
    // border: 1px solid rgba(0, 0, 0, 0.02);
    .q-table {
      display: block;
    }
    tr {
      margin-bottom: 1px;
    }
    tbody,
    tr,
    td {
      width: 100%;
      display: block;
      height: 25px;
    }

    .q-table__top {
      padding: 0px;
    }
    .q-table__control,
    label.q-field {
      width: 100%;
    }
    .q-table thead tr,
    .q-table tbody td {
      height: auto;
    }

    .q-table__top {
      border-bottom: unset;
    }
  }
  .field-table {
    width: 100%;
  }

  .field_list {
    padding: 0px;
    margin-bottom: 0.125rem;
    position: relative;
    overflow: visible;
    cursor: default;

    .field_overlay {
      justify-content: space-between;
      background-color: transparent;
      transition: all 0.3s ease;
      padding: 0px 10px;
      align-items: center;
      position: absolute;
      line-height: 2rem;
      overflow: hidden;
      inset: 0;
      display: flex;
      z-index: 1;
      width: 100%;
      border-radius: 0px;
      height: 25px;

      .field_icons {
        padding: 0 0 0 0.25rem;
        transition: all 0.3s ease;
        background-color: white;
        position: absolute;
        z-index: 3;
        opacity: 0;
        right: 0;

        .q-icon {
          cursor: pointer;
        }
      }

      .field_label {
        pointer-events: none;
        font-size: 0.825rem;
        position: relative;
        display: inline;
        z-index: 2;
        left: 0;
        // text-transform: capitalize;
      }
    }

    &.selected {
      .field_overlay {
        background-color: rgba(89, 96, 178, 0.3);

        .field_icons {
          opacity: 0;
        }
      }
      &:hover {
        .field_overlay {
          box-shadow: 0px 4px 15px rgba(0, 0, 0, 0.17);
          background-color: white;

          .field_icons {
            background-color: white;
          }
        }
      }
    }
    &:hover {
      .field_overlay {
        box-shadow: 0px 4px 15px rgba(0, 0, 0, 0.17);

        .field_icons {
          background-color: white;
          opacity: 1;
        }
      }
    }
  }
}
.q-item {
  color: $dark-page;
  min-height: 1.3rem;
  padding: 5px 10px;

  &__label {
    font-size: 0.75rem;
  }

  &.q-manual-focusable--focused > .q-focus-helper {
    background: none !important;
    opacity: 0.3 !important;
  }

  &.q-manual-focusable--focused > .q-focus-helper,
  &--active {
    background-color: $selected-list-bg !important;
  }

  &.q-manual-focusable--focused > .q-focus-helper,
  &:hover,
  &--active {
    color: $primary;
  }
}
.q-field--dense .q-field__before,
.q-field--dense .q-field__prepend {
  padding: 0px 0px 0px 0px;
  height: auto;
  line-height: auto;
}
.q-field__native,
.q-field__input {
  padding: 0px 0px 0px 0px;
}

.q-field--dense .q-field__label {
  top: 5px;
}
.q-field--dense .q-field__control,
.q-field--dense .q-field__marginal {
  height: 34px;
}
</style>
