<!-- Copyright 2022 Zinc Labs Inc. and Contributors

 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at

     http:www.apache.org/licenses/LICENSE-2.0

 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License. 
-->

<!-- eslint-disable vue/v-on-event-hyphenation -->
<template>
  <div
    :class="position === 'bottom' ? 'q-py-sm' : 'q-pt-sm'"
    class="q-table__control full-width row justify-between"
  >
    <div v-if="position === 'bottom' && maxRecords" class="max-result">
      <span class="q-table__bottom-item">{{ t("search.maxRecords") }}</span>
      <q-input
        v-model="maxRecords"
        filled
        dense
        class="max-records-input"
        @blur="changeMaxRecordToReturn"
      />
    </div>
    <div
      v-if="position === 'top' && pageTitle"
      class="text-bold row items-center"
    >
      <q-btn
        v-if="
          collapsibleIcon === 'show' &&
          seachCollapseImage == 'collapse_sidebar_icon'
        "
        icon="img:/src/assets/images/common/collapse_sidebar_icon.svg"
        class="q-mr-sm"
        size="sm"
        round
        flat
        @click="toggleSidePanel"
      />
      <q-btn
        v-if="
          collapsibleIcon === 'show' &&
          seachCollapseImage == 'expand_sidebar_icon'
        "
        icon="img:/src/assets/images/common/expand_sidebar_icon.svg"
        class="q-mr-sm"
        size="sm"
        round
        flat
        @click="toggleSidePanel"
      />
      <div class="q-ml-xs">
        {{ resultTotal }}
        {{ pageTitle.slice(-1) == "s" ? pageTitle.slice(0, -1) : pageTitle }}(s)
      </div>
    </div>
    <div class="q-table__control q-ml-auto">
      <span class="q-table__bottom-item">
        {{ t("search.showing") }}
        {{ (scope.pagination.page - 1) * scope.pagination.rowsPerPage + 1 }} -
        {{
          scope.pagination.page * scope.pagination.rowsPerPage >= resultTotal
            ? resultTotal
            : scope.pagination.page * scope.pagination.rowsPerPage
        }}
        {{ t("search.of") }} {{ resultTotal }}

        <!-- {{ (scope.pagination.page - 1) * scope.pagination.rowsPerPage + 1 }}-{{
          scope.pagination.page * scope.pagination.rowsPerPage
        }}
        of {{ resultTotal }} -->
      </span>

      <div v-if="position === 'bottom'" class="flex items-center">
        <q-separator vertical inset class="q-mr-md" />

        <span class="q-table__bottom-item">
          {{ t("search.recordsPerPage") }}
        </span>
        <q-select
          v-model="scope.pagination.rowsPerPage"
          class="q-mr-md"
          borderless
          size="sm"
          dense
          :options="perPageOptions"
          @update:modelValue="changePagination"
        />
      </div>

      <q-btn-group>
        <q-btn
          icon="chevron_left"
          :text-color="scope.isFirstPage ? '$light-text2' : '$dark'"
          class="pageNav"
          color="#FAFBFD"
          size="sm"
          flat
          :disable="scope.isFirstPage"
          @click="scope.prevPage"
        />
        <q-separator vertical />
        <q-btn
          icon="chevron_right"
          :text-color="scope.isLastPage ? '$light-text2' : '$dark'"
          class="pageNav"
          color="#FAFBFD"
          size="sm"
          flat
          :disable="scope.isLastPage"
          @click="scope.nextPage"
        />
      </q-btn-group>
    </div>
  </div>
</template>

<script lang="ts">
// @ts-nocheck
import { defineComponent, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useRouter } from "vue-router";

export default defineComponent({
  name: "QTablePagination",
  // eslint-disable-next-line vue/require-prop-types
  props: [
    "scope",
    "pageTitle",
    "resultTotal",
    "maxRecordToReturn",
    "changeRecordPerPage",
    "perPageOptions",
    "position",
    "collapsibleIcon",
  ] as string[],
  emits: ["update", "update:maxRecordToReturn", "update:changeRecordPerPage"],
  setup(props, { emit }) {
    const { t } = useI18n();
    const router = useRouter();
    const maxRecords = ref(props.maxRecordToReturn);
    const store = useStore();
    const seachCollapseImage: any = ref("collapse_sidebar_icon");

    const changePagination = (val: any) => {
      emit("update:changeRecordPerPage", val);
    };

    const changeMaxRecordToReturn = (e: { target: { value: any } }) => {
      emit("update:maxRecordToReturn", e.target.value);
    };

    const toggleSidePanel = () => {
      store.dispatch(
        "setSearchCollapseToggle",
        store.state.searchCollapsibleSection == 0 ? 20 : 0
      );
    };

    return {
      t,
      store,
      router,
      maxRecords,
      toggleSidePanel,
      seachCollapseImage,
      changePagination,
      changeMaxRecordToReturn,
    };
  },
  computed: {
    sidebarIcon() {
      return this.store.state.searchCollapsibleSection;
    },
  },
  watch: {
    sidebarIcon(newVal: any, oldVal: any) {
      if (newVal != oldVal && this.router.currentRoute.value.name == "logs") {
        if (this.store.state.searchCollapsibleSection == 0) {
          this.seachCollapseImage = "expand_sidebar_icon";
        } else {
          this.seachCollapseImage = "collapse_sidebar_icon";
        }
      }
    },
  },
});
</script>

<style lang="scss">
.footer-text {
  margin-right: 1rem;
  font-size: 0.75rem;
  font-weight: 600;
  color: $dark;
}
.q-table__bottom-item {
  @extend .footer-text;
}
.q-select .q-field {
  &__native {
    @extend .footer-text;
    text-align: center;
    margin-right: 0;
  }
  &__append {
    padding: 0;
  }
}

.pageNav {
  padding: 0.125rem 0.5rem;
}

.max-result {
  justify-content: center;
  align-items: center;
  white-space: nowrap;
  display: flex;
  width: 200px;

  .q-field {
    max-width: 3.5rem;
  }

  .max-records-input {
    .q-field {
      &__control {
        background-color: #fafbfd !important;
        max-width: 2.5rem;
        height: 1.5rem;
        padding: 0;
      }
      &__native {
        font-size: 0.75rem;
        text-align: center;
        font-weight: 600;
        color: $dark;
        padding: 0;
      }
    }
  }
}
</style>
